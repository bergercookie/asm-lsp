use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::fs::{create_dir_all, File};
use std::io::BufRead;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use compile_commands::{CompilationDatabase, CompileCommand, SourceFile};
use dirs::config_dir;
use log::{error, info, log, log_enabled, warn};
use lsp_textdocument::FullTextDocument;
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, CompletionParams, CompletionTriggerKind,
    Diagnostic, DocumentSymbol, DocumentSymbolParams, Documentation, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverContents, HoverParams, InitializeParams, Location,
    MarkupContent, MarkupKind, Position, Range, ReferenceParams, SignatureHelp,
    SignatureHelpParams, SignatureInformation, SymbolKind, TextDocumentContentChangeEvent,
    TextDocumentPositionParams, Uri,
};
use once_cell::sync::Lazy;
use regex::Regex;
use symbolic::common::{Language, Name, NameMangling};
use symbolic_demangle::{Demangle, DemangleOptions};
use tree_sitter::InputEdit;

use crate::types::Column;
use crate::{
    Arch, ArchOrAssembler, Assembler, Completable, Hoverable, Instruction, NameToInstructionMap,
    TargetConfig, TreeEntry,
};

/// Find the start and end indices of a word inside the given line
/// Borrowed from RLS
/// `additional_chars` parameter allows specifying additional legal "word"
/// characters besides the default alphanumeric and '_'
#[must_use]
pub fn find_word_at_pos(line: &str, col: Column, additional_chars: &str) -> (Column, Column) {
    let line_ = format!("{line} ");
    let is_ident_char = |c: char| {
        c.is_alphanumeric() || c == '_' || additional_chars.chars().any(|word_char| word_char == c)
    };

    let start = line_
        .chars()
        .enumerate()
        .take(col)
        .filter(|&(_, c)| !is_ident_char(c))
        .last()
        .map_or(0, |(i, _)| i + 1);

    #[allow(clippy::filter_next)]
    let mut end = line_
        .chars()
        .enumerate()
        .skip(col)
        .filter(|&(_, c)| !is_ident_char(c));

    let end = end.next();
    (start, end.map_or(col, |(i, _)| i))
}

/// Returns the word undernearth the cursor given the specified `TextDocumentPositionParams`
///
/// # Errors
///
/// Will return `Err` if the file cannot be opened
///
/// # Panics
///
/// Will panic if the position parameters specify a line past the end of the file's
/// contents
pub fn get_word_from_file_params(
    pos_params: &TextDocumentPositionParams,
    extra_chars: &str,
) -> Result<String> {
    let uri = &pos_params.text_document.uri;
    let line = pos_params.position.line as usize;
    let col = pos_params.position.character as usize;

    let filepath = PathBuf::from(uri.as_str());
    match filepath.canonicalize() {
        Ok(file) => {
            let file = match File::open(file) {
                Ok(opened) => opened,
                Err(e) => return Err(anyhow!("Couldn't open file -> {:?} -- Error: {e}", uri)),
            };
            let buf_reader = std::io::BufReader::new(file);

            let line_conts = buf_reader.lines().nth(line).unwrap().unwrap();
            let (start, end) = find_word_at_pos(&line_conts, col, extra_chars);
            Ok(String::from(&line_conts[start..end]))
        }
        Err(e) => Err(anyhow!("Filepath get error -- Error: {e}")),
    }
}

/// Returns a string slice to the word in doc specified by the position params
///
/// The `extra_chars` param allows specifying extra chars to be considered as
/// valid word chars, in addition to the default alphanumeric and '_' chars
// extra_chars is used when grabbing filenames from a document, as '.' isn't
// normally considered a valid "word" char
#[must_use]
pub fn get_word_from_pos_params<'a>(
    doc: &'a FullTextDocument,
    pos_params: &TextDocumentPositionParams,
    extra_chars: &str,
) -> &'a str {
    let line_contents = doc.get_content(Some(Range {
        start: Position {
            line: pos_params.position.line,
            character: 0,
        },
        end: Position {
            line: pos_params.position.line,
            character: u32::MAX,
        },
    }));

    let (word_start, word_end) = find_word_at_pos(
        line_contents,
        pos_params.position.character as usize,
        extra_chars,
    );
    &line_contents[word_start..word_end]
}

/// Fetches default include directories, as well as any additional directories
/// as specified by a `compile_commands.json` or `compile_flags.txt` file in the
/// appropriate location
///
/// # Panics
#[must_use]
pub fn get_include_dirs(compile_cmds: &CompilationDatabase) -> HashMap<SourceFile, Vec<PathBuf>> {
    let mut include_map = HashMap::from([(SourceFile::All, Vec::new())]);

    let global_dirs = include_map.get_mut(&SourceFile::All).unwrap();
    for dir in get_default_include_dirs() {
        global_dirs.push(dir);
    }

    for (source_file, ref dir) in get_additional_include_dirs(compile_cmds) {
        include_map
            .entry(source_file)
            .and_modify(|dirs| dirs.push(dir.to_owned()))
            .or_insert(vec![dir.to_owned()]);
    }

    info!("Include directory map: {:?}", include_map);

    include_map
}

/// Returns a vector of default #include directories
#[must_use]
fn get_default_include_dirs() -> Vec<PathBuf> {
    let mut include_dirs = HashSet::new();
    // repeat "cpp" and "clang" so that each command can be run with
    // both set of args specified in `cmd_args`
    let cmds = &["cpp", "cpp", "clang", "clang"];
    let cmd_args = &[
        ["-v", "-E", "-x", "c", "/dev/null", "-o", "/dev/null"],
        ["-v", "-E", "-x", "c++", "/dev/null", "-o", "/dev/null"],
    ];

    for (cmd, args) in cmds.iter().zip(cmd_args.iter().cycle()) {
        if let Ok(cmd_output) = std::process::Command::new(cmd)
            .args(args)
            .stderr(std::process::Stdio::piped())
            .output()
        {
            if cmd_output.status.success() {
                let output_str: String = String::from_utf8(cmd_output.stderr).unwrap_or_default();

                output_str
                    .lines()
                    .skip_while(|line| !line.contains("#include \"...\" search starts here:"))
                    .skip(1)
                    .take_while(|line| {
                        !(line.contains("End of search list.")
                            || line.contains("#include <...> search starts here:"))
                    })
                    .filter_map(|line| PathBuf::from(line.trim()).canonicalize().ok())
                    .for_each(|path| {
                        include_dirs.insert(path);
                    });

                output_str
                    .lines()
                    .skip_while(|line| !line.contains("#include <...> search starts here:"))
                    .skip(1)
                    .take_while(|line| !line.contains("End of search list."))
                    .filter_map(|line| PathBuf::from(line.trim()).canonicalize().ok())
                    .for_each(|path| {
                        include_dirs.insert(path);
                    });
            }
        }
    }

    include_dirs.iter().cloned().collect::<Vec<PathBuf>>()
}

/// Returns a vector of source files and their associated additional include directories,
/// as specified by `compile_cmds`
#[must_use]
fn get_additional_include_dirs(compile_cmds: &CompilationDatabase) -> Vec<(SourceFile, PathBuf)> {
    let mut additional_dirs = Vec::new();

    for entry in compile_cmds {
        let Ok(entry_dir) = entry.directory.canonicalize() else {
            continue;
        };

        let source_file = match &entry.file {
            SourceFile::All => SourceFile::All,
            SourceFile::File(file) => {
                if file.is_absolute() {
                    entry.file.clone()
                } else if let Ok(dir) = entry_dir.join(file).canonicalize() {
                    SourceFile::File(dir)
                } else {
                    continue;
                }
            }
        };

        let mut check_dir = false;
        if let Some(args) = &entry.arguments {
            // `arguments` run as the compilation step for the translation unit `file`
            // We will try to canonicalize non-absolute paths as relative to `file`,
            // but this isn't possible if we have a SourceFile::All. Just don't
            // add the include directory and issue a warning in this case
            for arg in args.iter().map(|arg| arg.trim()) {
                if check_dir {
                    // current arg is preceeded by lone '-I'
                    let dir = PathBuf::from(arg);
                    if dir.is_absolute() {
                        additional_dirs.push((source_file.clone(), dir));
                    } else if let SourceFile::File(ref source_path) = source_file {
                        if let Ok(full_include_path) = source_path.join(dir).canonicalize() {
                            additional_dirs.push((source_file.clone(), full_include_path));
                        }
                    } else {
                        warn!("Additional relative include directories cannot be extracted for a compilation database entry targeting 'All'");
                    }
                    check_dir = false;
                } else if arg.eq("-I") {
                    // -Irelative is stored as two separate args if parsed from `compile_flags.txt`
                    check_dir = true;
                } else if arg.len() > 2 && arg.starts_with("-I") {
                    // '-Irelative'
                    let dir = PathBuf::from(&arg[2..]);
                    if dir.is_absolute() {
                        additional_dirs.push((source_file.clone(), dir));
                    } else if let SourceFile::File(ref source_path) = source_file {
                        if let Ok(full_include_path) = source_path.join(dir).canonicalize() {
                            additional_dirs.push((source_file.clone(), full_include_path));
                        }
                    } else {
                        warn!("Additional relative include directories cannot be extracted for a compilation database entry targeting 'All'");
                    }
                }
            }
        } else if entry.command.is_some() {
            if let Some(args) = entry.args_from_cmd() {
                for arg in args {
                    if arg.starts_with("-I") && arg.len() > 2 {
                        // "All paths specified in the `command` or `file` fields must be either absolute or relative to..." the `directory` field
                        let incl_path = PathBuf::from(&arg[2..]);
                        if incl_path.is_absolute() {
                            additional_dirs.push((source_file.clone(), incl_path));
                        } else {
                            let dir = entry_dir.join(incl_path);
                            if let Ok(full_include_path) = dir.canonicalize() {
                                additional_dirs.push((source_file.clone(), full_include_path));
                            }
                        }
                    }
                }
            }
        }
    }

    additional_dirs
}

/// Attempts to find either the `compile_commands.json` or `compile_flags.txt`
/// file in the project's build directory, returning either as a `CompilationDatabase`
/// object
///
/// If both are present, `compile_commands.json` will override `compile_flags.txt`
pub fn get_compile_cmds(params: &InitializeParams) -> Option<CompilationDatabase> {
    if let Some(mut path) = get_project_root(params) {
        // "The convention is to name the file compile_commands.json and put it at the top of the
        // build directory."
        path.push("build");

        // first check for compile_commands.json
        let cmp_cmd_path = path.join("compile_commands.json");
        if let Ok(conts) = std::fs::read_to_string(cmp_cmd_path) {
            if let Ok(cmds) = serde_json::from_str(&conts) {
                return Some(cmds);
            }
        }
        // then check for compile_flags.txt
        let cmp_flag_path = path.join("compile_flags.txt");
        if let Ok(conts) = std::fs::read_to_string(cmp_flag_path) {
            return Some(compile_commands::from_compile_flags_txt(&path, &conts));
        }
    }

    None
}

/// Attempts to run the given compile command and parses the resulting output. Any
/// relevant output will be translated into a `Diagnostic` object and pushed into
/// `diagnostics`
pub fn apply_compile_cmd(diagnostics: &mut Vec<Diagnostic>, compile_cmd: &CompileCommand) {
    // TODO: Consolidate this logic, a little tricky because we need to capture
    // compile_cmd.arguments by reference, but we get an owned Vec out of args_from_cmd()...
    if let Some(ref args) = compile_cmd.arguments {
        if args.len() < 2 {
            return;
        }
        let output = match Command::new(&args[0]).args(&args[1..]).output() {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to launch compile command process -- Error: {e}");
                return;
            }
        };
        if let Ok(output_str) = String::from_utf8(output.stderr) {
            get_diagnostics(diagnostics, &output_str);
        }
    } else if let Some(args) = compile_cmd.args_from_cmd() {
        if args.len() < 2 {
            return;
        }
        let output = match Command::new(&args[0]).args(&args[1..]).output() {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to launch compile command process -- Error: {e}");
                return;
            }
        };
        if let Ok(output_str) = String::from_utf8(output.stderr) {
            get_diagnostics(diagnostics, &output_str);
        }
    }
}

/// Attempts to parse `tool_output`, translating it into `Diagnostic` objects
/// and placing them into `diagnostics`
///
/// Looks for diagnostics of the following form:
///
/// <file name>:<line number>: Error: <Error message>
///
/// As more assemblers are incorporated, this can be updated
///
/// # Panics
fn get_diagnostics(diagnostics: &mut Vec<Diagnostic>, tool_output: &str) {
    static DIAG_REG: Lazy<Regex> = Lazy::new(|| Regex::new(r"^.*:(\d+):\s+(.*)$").unwrap());

    for line in tool_output.lines() {
        if let Some(caps) = DIAG_REG.captures(line) {
            if caps.len() < 3 {
                // the entire capture is always at the 0th index
                continue;
            }
            let Ok(line_number) = caps[1].parse::<u32>() else {
                continue;
            };
            let err_msg = &caps[2];
            diagnostics.push(Diagnostic::new_simple(
                Range {
                    start: Position {
                        line: line_number - 1,
                        character: 0,
                    },
                    end: Position {
                        line: line_number - 1,
                        character: 0,
                    },
                },
                String::from(err_msg),
            ));
        }
    }
}

/// Function allowing us to connect tree sitter's logging with the log crate
pub fn tree_sitter_logger(log_type: tree_sitter::LogType, message: &str) {
    // map tree-sitter log types to log levels, for now set everything to Trace
    let log_level = match log_type {
        tree_sitter::LogType::Parse | tree_sitter::LogType::Lex => log::Level::Trace,
    };

    // tree-sitter logs are incredibly verbose, only forward them to the logger
    // if we *really* need to see what's going on
    if log_enabled!(log_level) {
        log!(log_level, "{}", message);
    }
}

/// Convert an `lsp_types::TextDocumentContentChangeEvent` to a `tree_sitter::InputEdit`
///
/// # Errors
///
/// Returns `Err` if `change.range` is `None`, or if a `usize`->`u32` numeric conversion
/// failed
pub fn text_doc_change_to_ts_edit(
    change: &TextDocumentContentChangeEvent,
    doc: &FullTextDocument,
) -> Result<InputEdit> {
    let range = change.range.ok_or(anyhow!("Invalid edit range"))?;
    let start = range.start;
    let end = range.end;

    let start_byte = doc.offset_at(start) as usize;
    let new_end_byte = start_byte + change.text.len();
    let new_end_pos = doc.position_at(u32::try_from(new_end_byte)?);

    Ok(tree_sitter::InputEdit {
        start_byte,
        old_end_byte: doc.offset_at(end) as usize,
        new_end_byte,
        start_position: tree_sitter::Point {
            row: start.line as usize,
            column: start.character as usize,
        },
        old_end_position: tree_sitter::Point {
            row: end.line as usize,
            column: end.character as usize,
        },
        new_end_position: tree_sitter::Point {
            row: new_end_pos.line as usize,
            column: new_end_pos.character as usize,
        },
    })
}

/// Given a `NameTo_SomeItem_` map, returns a `Vec<CompletionItem>` for the items
/// contained within the map
#[must_use]
pub fn get_completes<T: Completable, U: ArchOrAssembler>(
    map: &HashMap<(U, &str), T>,
    kind: Option<CompletionItemKind>,
) -> Vec<CompletionItem> {
    map.iter()
        .map(|((_arch_or_asm, name), item_info)| {
            let value = format!("{item_info}");

            CompletionItem {
                label: (*name).to_string(),
                kind,
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                })),
                ..Default::default()
            }
        })
        .collect()
}

#[must_use]
pub fn get_hover_resp<T: Hoverable, U: Hoverable, V: Hoverable>(
    params: &HoverParams,
    word: &str,
    file_word: &str,
    instruction_map: &HashMap<(Arch, &str), T>,
    register_map: &HashMap<(Arch, &str), U>,
    directive_map: &HashMap<(Assembler, &str), V>,
    include_dirs: &HashMap<SourceFile, Vec<PathBuf>>,
) -> Option<Hover> {
    let instr_lookup = lookup_hover_resp_by_arch(word, instruction_map);
    if instr_lookup.is_some() {
        return instr_lookup;
    }

    let directive_lookup = lookup_hover_resp_by_assembler(word, directive_map);
    if directive_lookup.is_some() {
        return directive_lookup;
    }

    let reg_lookup = lookup_hover_resp_by_arch(word, register_map);
    if reg_lookup.is_some() {
        return reg_lookup;
    }

    let demang = get_demangle_resp(word);
    if demang.is_some() {
        return demang;
    }

    let include_path = get_include_resp(
        &params.text_document_position_params.text_document.uri,
        file_word,
        include_dirs,
    );
    if include_path.is_some() {
        return include_path;
    }

    None
}

fn lookup_hover_resp_by_arch<T: Hoverable>(
    word: &str,
    map: &HashMap<(Arch, &str), T>,
) -> Option<Hover> {
    // switch over to vec?
    let (x86_resp, x86_64_resp, z80_resp, arm_resp) = search_for_hoverable_by_arch(word, map);
    match (
        x86_resp.is_some(),
        x86_64_resp.is_some(),
        z80_resp.is_some(),
        arm_resp.is_some(),
    ) {
        (true, _, _, _) | (_, true, _, _) | (_, _, true, _) | (_, _, _, true) => {
            let mut value = String::new();
            if let Some(x86_resp) = x86_resp {
                value += &format!("{x86_resp}");
            }
            if let Some(x86_64_resp) = x86_64_resp {
                value += &format!(
                    "{}{}",
                    if value.is_empty() { "" } else { "\n\n" },
                    x86_64_resp
                );
            }
            if let Some(z80_resp) = z80_resp {
                value += &format!("{}{}", if value.is_empty() { "" } else { "\n\n" }, z80_resp);
            }
            if let Some(arm_resp) = arm_resp {
                value += &format!("{}{}", if value.is_empty() { "" } else { "\n\n" }, arm_resp);
            }
            Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: None,
            })
        }
        _ => {
            // don't know of this word
            None
        }
    }
}

fn lookup_hover_resp_by_assembler<T: Hoverable>(
    word: &str,
    map: &HashMap<(Assembler, &str), T>,
) -> Option<Hover> {
    let (gas_resp, go_resp) = search_for_hoverable_by_assembler(word, map);

    match (gas_resp.is_some(), go_resp.is_some()) {
        (true, _) | (_, true) => {
            let mut value = String::new();
            if let Some(gas_resp) = gas_resp {
                value += &format!("{gas_resp}");
            }
            if let Some(go_resp) = go_resp {
                value += &format!(
                    "{}{}",
                    if gas_resp.is_some() { "\n\n" } else { "" },
                    go_resp
                );
            }
            Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: None,
            })
        }
        _ => {
            // don't know of this word
            None
        }
    }
}

fn get_demangle_resp(word: &str) -> Option<Hover> {
    let name = Name::new(word, NameMangling::Mangled, Language::Unknown);
    let demangled = name.demangle(DemangleOptions::complete());
    if let Some(demang) = demangled {
        let value = demang.to_string();
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: None,
        });
    }

    None
}

fn get_include_resp(
    source_file: &Uri,
    filename: &str,
    include_dirs: &HashMap<SourceFile, Vec<PathBuf>>,
) -> Option<Hover> {
    let mut paths = String::new();

    let mut dir_iter: Box<dyn Iterator<Item = &PathBuf>> = match include_dirs.get(&SourceFile::All)
    {
        Some(dirs) => Box::new(dirs.iter()),
        None => Box::new(std::iter::empty()),
    };

    if let Ok(src_path) = PathBuf::from(source_file.as_str()).canonicalize() {
        if let Some(dirs) = include_dirs.get(&SourceFile::File(src_path)) {
            dir_iter = Box::new(dir_iter.chain(dirs.iter()));
        }
    }

    for dir in dir_iter {
        match std::fs::read_dir(dir) {
            Ok(dir_reader) => {
                for file in dir_reader {
                    match file {
                        Ok(f) => {
                            if f.file_name() == filename {
                                paths += &format!("file://{}\n", f.path().display());
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to read item in {} - Error {e}",
                                dir.as_path().display()
                            );
                        }
                    };
                }
            }
            Err(e) => {
                error!(
                    "Failed to create directory reader for {} - Error {e}",
                    dir.as_path().display()
                );
            }
        }
    }

    if paths.is_empty() {
        None
    } else {
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: paths,
            }),
            range: None,
        })
    }
}

/// Filter out duplicate completion suggestions
fn filtered_comp_list(comps: &[CompletionItem]) -> Vec<CompletionItem> {
    let mut seen = HashSet::new();

    comps
        .iter()
        .filter(|comp_item| {
            if seen.contains(&comp_item.label) {
                false
            } else {
                seen.insert(&comp_item.label);
                true
            }
        })
        .cloned()
        .collect()
}

macro_rules! cursor_matches {
    ($cursor_line:expr,$cursor_char:expr,$query_start:expr,$query_end:expr) => {{
        $query_start.row == $cursor_line
            && $query_end.row == $cursor_line
            && $query_start.column <= $cursor_char
            && $query_end.column >= $cursor_char
    }};
}

pub fn get_comp_resp(
    curr_doc: &str,
    tree_entry: &mut TreeEntry,
    params: &CompletionParams,
    instr_comps: &[CompletionItem],
    dir_comps: &[CompletionItem],
    reg_comps: &[CompletionItem],
) -> Option<CompletionList> {
    let cursor_line = params.text_document_position.position.line as usize;
    let cursor_char = params.text_document_position.position.character as usize;

    if let Some(ctx) = params.context.as_ref() {
        if ctx.trigger_kind == CompletionTriggerKind::TRIGGER_CHARACTER {
            match ctx
                .trigger_character
                .as_ref()
                .map(std::convert::AsRef::as_ref)
            {
                // prepend GAS registers with "%"
                Some("%") => {
                    return Some(CompletionList {
                        is_incomplete: true,
                        items: filtered_comp_list(reg_comps),
                    });
                }
                // prepend GAS directives with "."
                Some(".") => {
                    return Some(CompletionList {
                        is_incomplete: true,
                        items: filtered_comp_list(dir_comps),
                    });
                }
                _ => {}
            }
        }
    }

    // TODO: filter register completions by width allowed by corresponding instruction
    tree_entry.tree = tree_entry.parser.parse(curr_doc, tree_entry.tree.as_ref());
    if let Some(ref tree) = tree_entry.tree {
        let mut line_cursor = tree_sitter::QueryCursor::new();
        line_cursor.set_point_range(std::ops::Range {
            start: tree_sitter::Point {
                row: cursor_line,
                column: 0,
            },
            end: tree_sitter::Point {
                row: cursor_line,
                column: usize::MAX,
            },
        });
        let curr_doc = curr_doc.as_bytes();

        static QUERY_DIRECTIVE: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(
                &tree_sitter_asm::language(),
                "(meta kind: (meta_ident) @directive)",
            )
            .unwrap()
        });
        let matches_iter = line_cursor.matches(&QUERY_DIRECTIVE, tree.root_node(), curr_doc);

        for match_ in matches_iter {
            let caps = match_.captures;
            for cap in caps {
                let arg_start = cap.node.range().start_point;
                let arg_end = cap.node.range().end_point;
                if cursor_matches!(cursor_line, cursor_char, arg_start, arg_end) {
                    let items = filtered_comp_list(dir_comps);
                    return Some(CompletionList {
                        is_incomplete: true,
                        items,
                    });
                }
            }
        }

        // tree-sitter-asm currently parses label arguments to instructions as *registers*
        // We'll collect all of labels in the document (that are being parsed as labels, at least)
        // and suggest those along with the register completions

        // need a separate cursor to search the entire document
        let mut doc_cursor = tree_sitter::QueryCursor::new();
        static QUERY_LABEL: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(&tree_sitter_asm::language(), "(label (ident) @label)").unwrap()
        });
        let captures = doc_cursor.captures(&QUERY_LABEL, tree.root_node(), curr_doc);
        let mut labels = HashSet::new();
        for caps in captures.map(|c| c.0) {
            for cap in caps.captures {
                match cap.node.utf8_text(curr_doc) {
                    Ok(text) => _ = labels.insert(text),
                    Err(_) => continue,
                }
            }
        }

        static QUERY_INSTR_ANY: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(
                &tree_sitter_asm::language(),
                "[
                    (instruction kind: (word) @instr_name)
                    (
                        instruction kind: (word) @instr_name
                            [
                                (
                                    [
                                     (ident (reg) @r1)
                                     (ptr (int) (reg) @r1)
                                     (ptr (reg) @r1)
                                     (ptr (int))
                                     (ptr)
                                    ]
                                    [
                                     (ident (reg) @r2)
                                     (ptr (int) (reg) @r2)
                                     (ptr (reg) @r2)
                                     (ptr (int))
                                     (ptr)
                                    ]
                                )
                                (
                                    [
                                     (ident (reg) @r1)
                                     (ptr (int) (reg) @r1)
                                     (ptr (reg) @r1)
                                    ]
                                )
                            ]
                    )
                ]",
            )
            .unwrap()
        });

        let matches_iter = line_cursor.matches(&QUERY_INSTR_ANY, tree.root_node(), curr_doc);
        for match_ in matches_iter {
            let caps = match_.captures;
            for (cap_num, cap) in caps.iter().enumerate() {
                let arg_start = cap.node.range().start_point;
                let arg_end = cap.node.range().end_point;
                if cursor_matches!(cursor_line, cursor_char, arg_start, arg_end) {
                    // an instruction is always capture #0 for this query, any capture
                    // number after must be a register or label
                    let is_instr = cap_num == 0;
                    let mut items =
                        filtered_comp_list(if is_instr { instr_comps } else { reg_comps });
                    if !is_instr {
                        items.append(
                            &mut labels
                                .iter()
                                .map(|l| CompletionItem {
                                    label: (*l).to_string(),
                                    kind: Some(CompletionItemKind::VARIABLE),
                                    ..Default::default()
                                })
                                .collect(),
                        );
                    }
                    return Some(CompletionList {
                        is_incomplete: true,
                        items,
                    });
                }
            }
        }
    }

    None
}

fn lsp_pos_of_point(pos: tree_sitter::Point) -> lsp_types::Position {
    Position {
        line: pos.row as u32,
        character: pos.column as u32,
    }
}

/// Get a tree of symbols describing the document's structure.
pub fn get_document_symbols(
    curr_doc: &str,
    tree_entry: &mut TreeEntry,
    _params: &DocumentSymbolParams,
) -> Option<Vec<DocumentSymbol>> {
    tree_entry.tree = tree_entry.parser.parse(curr_doc, tree_entry.tree.as_ref());

    static LABEL_KIND_ID: Lazy<u16> =
        Lazy::new(|| tree_sitter_asm::language().id_for_node_kind("label", true));
    static IDENT_KIND_ID: Lazy<u16> =
        Lazy::new(|| tree_sitter_asm::language().id_for_node_kind("ident", true));

    /// Explore `node`, push immediate children into `res`.
    fn explore_node(curr_doc: &str, node: tree_sitter::Node, res: &mut Vec<DocumentSymbol>) {
        if node.kind_id() == *LABEL_KIND_ID {
            let mut children = vec![];
            let mut cursor = node.walk();

            // description for this node
            let mut descr = String::new();

            if cursor.goto_first_child() {
                loop {
                    let sub_node = cursor.node();
                    if sub_node.kind_id() == *IDENT_KIND_ID {
                        if let Ok(text) = sub_node.utf8_text(curr_doc.as_bytes()) {
                            descr = text.to_string();
                        }
                    }

                    explore_node(curr_doc, sub_node, &mut children);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }

            let range = lsp_types::Range::new(
                lsp_pos_of_point(node.start_position()),
                lsp_pos_of_point(node.end_position()),
            );

            #[allow(deprecated)]
            let doc = DocumentSymbol {
                name: descr,
                detail: None,
                kind: SymbolKind::FUNCTION,
                tags: None,
                deprecated: Some(false),
                range,
                selection_range: range,
                children: if children.is_empty() {
                    None
                } else {
                    Some(children)
                },
            };
            res.push(doc);
        } else {
            let mut cursor = node.walk();

            if cursor.goto_first_child() {
                loop {
                    explore_node(curr_doc, cursor.node(), res);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
        }
    }
    if let Some(ref tree) = tree_entry.tree {
        let mut res: Vec<DocumentSymbol> = vec![];
        let mut cursor = tree.walk();
        loop {
            explore_node(curr_doc, cursor.node(), &mut res);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        Some(res)
    } else {
        None
    }
}

pub fn get_sig_help_resp(
    curr_doc: &str,
    params: &SignatureHelpParams,
    tree_entry: &mut TreeEntry,
    instr_info: &NameToInstructionMap,
) -> Option<SignatureHelp> {
    let cursor_line = params.text_document_position_params.position.line as usize;

    tree_entry.tree = tree_entry.parser.parse(curr_doc, tree_entry.tree.as_ref());
    if let Some(ref tree) = tree_entry.tree {
        let mut line_cursor = tree_sitter::QueryCursor::new();
        line_cursor.set_point_range(std::ops::Range {
            start: tree_sitter::Point {
                row: cursor_line,
                column: 0,
            },
            end: tree_sitter::Point {
                row: cursor_line,
                column: usize::MAX,
            },
        });
        let curr_doc = curr_doc.as_bytes();

        // Instruction with any (including zero) argument(s)
        static QUERY_INSTR_ANY_ARGS: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(
                &tree_sitter_asm::language(),
                "(instruction kind: (word) @instr_name)",
            )
            .unwrap()
        });

        let matches: Vec<tree_sitter::QueryMatch<'_, '_>> = line_cursor
            .matches(&QUERY_INSTR_ANY_ARGS, tree.root_node(), curr_doc)
            .collect();
        if let Some(match_) = matches.first() {
            let caps = match_.captures;
            if caps.len() == 1 {
                if let Ok(instr_name) = caps[0].node.utf8_text(curr_doc) {
                    let mut value = String::new();
                    let mut has_x86 = false;
                    let mut has_x86_64 = false;
                    let mut has_z80 = false;
                    let mut has_arm = false;
                    let (x86_info, x86_64_info, z80_info, arm_info) =
                        search_for_hoverable_by_arch(instr_name, instr_info);
                    if let Some(sig) = x86_info {
                        for form in &sig.forms {
                            if let Some(ref gas_name) = form.gas_name {
                                if instr_name.eq_ignore_ascii_case(gas_name) {
                                    if !has_x86 {
                                        value += "**x86**\n";
                                        has_x86 = true;
                                    }
                                    value += &format!("{form}\n");
                                }
                            } else if let Some(ref go_name) = form.go_name {
                                if instr_name.eq_ignore_ascii_case(go_name) {
                                    if !has_x86 {
                                        value += "**x86**\n";
                                        has_x86 = true;
                                    }
                                    value += &format!("{form}\n");
                                }
                            }
                        }
                    }
                    if let Some(sig) = x86_64_info {
                        for form in &sig.forms {
                            if let Some(ref gas_name) = form.gas_name {
                                if instr_name.eq_ignore_ascii_case(gas_name) {
                                    if !has_x86_64 {
                                        value += "**x86_64**\n";
                                        has_x86_64 = true;
                                    }
                                    value += &format!("{form}\n");
                                }
                            } else if let Some(ref go_name) = form.go_name {
                                if instr_name.eq_ignore_ascii_case(go_name) {
                                    if !has_x86_64 {
                                        value += "**x86_64**\n";
                                        has_x86_64 = true;
                                    }
                                    value += &format!("{form}\n");
                                }
                            }
                        }
                    }
                    if let Some(sig) = z80_info {
                        for form in &sig.forms {
                            if let Some(ref z80_name) = form.z80_name {
                                if instr_name.eq_ignore_ascii_case(z80_name) {
                                    if !has_z80 {
                                        value += "**z80**\n";
                                        has_z80 = true;
                                    }
                                    value += &format!("{form}\n");
                                }
                            }
                        }
                    }
                    if let Some(sig) = arm_info {
                        for form in &sig.asm_templates {
                            if !has_arm {
                                value += "**arm**\n";
                                has_arm = true;
                            }
                            value += &format!("{form}\n");
                        }
                    }
                    if !value.is_empty() {
                        return Some(SignatureHelp {
                            signatures: vec![SignatureInformation {
                                label: instr_name.to_string(),
                                documentation: Some(Documentation::MarkupContent(MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value,
                                })),
                                parameters: None,
                                active_parameter: None,
                            }],
                            active_signature: None,
                            active_parameter: None,
                        });
                    }
                }
            }
        }
    }

    None
}

pub fn get_goto_def_resp(
    curr_doc: &FullTextDocument,
    tree_entry: &mut TreeEntry,
    params: &GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let doc = curr_doc.get_content(None);
    tree_entry.tree = tree_entry.parser.parse(doc, tree_entry.tree.as_ref());

    if let Some(ref tree) = tree_entry.tree {
        static QUERY_LABEL: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(&tree_sitter_asm::language(), "(label) @label").unwrap()
        });

        let is_not_ident_char = |c: char| !(c.is_alphanumeric() || c == '_');
        let mut cursor = tree_sitter::QueryCursor::new();
        let matches = cursor.matches(&QUERY_LABEL, tree.root_node(), doc.as_bytes());

        let word = get_word_from_pos_params(curr_doc, &params.text_document_position_params, "");

        for match_ in matches {
            for cap in match_.captures {
                let text = cap
                    .node
                    .utf8_text(doc.as_bytes())
                    .unwrap_or("")
                    .trim()
                    .trim_matches(is_not_ident_char);

                if word.eq(text) {
                    let start = cap.node.start_position();
                    let end = cap.node.end_position();
                    return Some(GotoDefinitionResponse::Scalar(Location {
                        uri: params
                            .text_document_position_params
                            .text_document
                            .uri
                            .clone(),
                        range: Range {
                            start: lsp_pos_of_point(start),
                            end: lsp_pos_of_point(end),
                        },
                    }));
                }
            }
        }
    }

    None
}

pub fn get_ref_resp(
    params: &ReferenceParams,
    curr_doc: &FullTextDocument,
    tree_entry: &mut TreeEntry,
) -> Vec<Location> {
    let mut refs: Vec<Location> = Vec::new();
    let doc = curr_doc.get_content(None);
    tree_entry.tree = tree_entry.parser.parse(doc, tree_entry.tree.as_ref());

    if let Some(ref tree) = tree_entry.tree {
        static QUERY_LABEL: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(
                &tree_sitter_asm::language(),
                "(label (ident (reg (word)))) @label",
            )
            .unwrap()
        });

        static QUERY_WORD: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(&tree_sitter_asm::language(), "(ident) @ident").unwrap()
        });

        let is_not_ident_char = |c: char| !(c.is_alphanumeric() || c == '_');
        let word = get_word_from_pos_params(curr_doc, &params.text_document_position, "");
        let uri = &params.text_document_position.text_document.uri;

        let mut cursor = tree_sitter::QueryCursor::new();
        if params.context.include_declaration {
            let label_matches = cursor.matches(&QUERY_LABEL, tree.root_node(), doc.as_bytes());
            for match_ in label_matches {
                for cap in match_.captures {
                    let text = cap
                        .node
                        .utf8_text(doc.as_bytes())
                        .unwrap_or("")
                        .trim()
                        .trim_matches(is_not_ident_char);

                    if word.eq(text) {
                        let start = lsp_pos_of_point(cap.node.start_position());
                        let end = lsp_pos_of_point(cap.node.end_position());
                        // TODO: Use a Hashset here once lsptextdocument bumps its
                        // dependency on lsp_types
                        if !refs.iter().any(|loc| loc.range.start == start) {
                            refs.push(Location {
                                uri: uri.clone(),
                                range: Range { start, end },
                            });
                        }
                    }
                }
            }
        }

        let word_matches = cursor.matches(&QUERY_WORD, tree.root_node(), doc.as_bytes());
        for match_ in word_matches {
            for cap in match_.captures {
                let text = cap
                    .node
                    .utf8_text(doc.as_bytes())
                    .unwrap_or("")
                    .trim()
                    .trim_matches(is_not_ident_char);

                if word.eq(text) {
                    let start = lsp_pos_of_point(cap.node.start_position());
                    let end = lsp_pos_of_point(cap.node.end_position());
                    // TODO: Use a Hashset here once lsptextdocument bumps its
                    // dependency on lsp_types
                    if !refs.iter().any(|loc| loc.range.start == start) {
                        refs.push(Location {
                            uri: uri.clone(),
                            range: Range { start, end },
                        });
                    }
                }
            }
        }
    }

    refs
}

// Note: Some issues here regarding entangled lifetimes
// -- https://github.com/rust-lang/rust/issues/80389
// If issue is resolved, can add a separate lifetime "'b" to "word"
// parameter such that 'a: 'b
// For now, using 'a for both isn't strictly necessary, but fits our use case
fn search_for_hoverable_by_arch<'a, T: Hoverable>(
    word: &'a str,
    map: &'a HashMap<(Arch, &str), T>,
) -> (Option<&'a T>, Option<&'a T>, Option<&'a T>, Option<&'a T>) {
    let x86_resp = map.get(&(Arch::X86, word));
    let x86_64_resp = map.get(&(Arch::X86_64, word));
    let z80_resp = map.get(&(Arch::Z80, word));
    let arm_resp = map.get(&(Arch::ARM, word));

    (x86_resp, x86_64_resp, z80_resp, arm_resp)
}

fn search_for_hoverable_by_assembler<'a, T: Hoverable>(
    word: &'a str,
    map: &'a HashMap<(Assembler, &str), T>,
) -> (Option<&'a T>, Option<&'a T>) {
    let gas_resp = map.get(&(Assembler::Gas, word));
    let go_resp = map.get(&(Assembler::Go, word));

    (gas_resp, go_resp)
}

/// Searches for global config in ~/.config/asm-lsp, then the project's directory
/// Project specific configs will override global configs
#[must_use]
pub fn get_target_config(params: &InitializeParams) -> TargetConfig {
    match (get_global_config(), get_project_config(params)) {
        (_, Some(proj_cfg)) => proj_cfg,
        (Some(global_cfg), None) => global_cfg,
        (None, None) => TargetConfig::default(), // default is to turn every non-z80 feature on
    }
}

/// Checks ~/.config/asm-lsp for a config file, creating directories along the way as necessary
fn get_global_config() -> Option<TargetConfig> {
    if let Some(mut cfg_path) = config_dir() {
        cfg_path.push("asm-lsp");
        let cfg_path_s = cfg_path.display();
        info!("Creating directories along {} as necessary...", cfg_path_s);
        match create_dir_all(&cfg_path) {
            Ok(()) => {
                cfg_path.push(".asm-lsp.toml");
                if let Ok(config) = std::fs::read_to_string(&cfg_path) {
                    let cfg_path_s = cfg_path.display();
                    match toml::from_str::<TargetConfig>(&config) {
                        Ok(config) => {
                            info!("Parsing global asm-lsp config from file -> {cfg_path_s}\n");
                            return Some(config);
                        }
                        Err(e) => {
                            error!(
                                "Failed to parse global config file {cfg_path_s} - Error: {e}\n"
                            );
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to create global config directory {cfg_path_s} - Error: {e}");
            }
        }
    }

    None
}

/// Attempts to find the project's root directory given its `InitializeParams`
// 1. if we have workspace folders, then iterate through them and assign the first valid one to
//    the root path
// 2. If we don't have worksace folders or none of them is a valid path, check the (deprecated)
//    root_uri field
fn get_project_root(params: &InitializeParams) -> Option<PathBuf> {
    // first check workspace folders
    if let Some(folders) = &params.workspace_folders {
        // if there's multiple, just visit in order until we find a valid folder
        for folder in folders {
            if let Ok(parsed) = PathBuf::from_str(folder.uri.path().as_str()) {
                if let Ok(parsed_path) = parsed.canonicalize() {
                    return Some(parsed_path);
                }
            }
        }
    }

    // if workspace folders weren't set or came up empty, we check the root_uri
    #[allow(deprecated)]
    if let Some(root_uri) = &params.root_uri {
        if let Ok(parsed) = PathBuf::from_str(root_uri.path().as_str()) {
            if let Ok(parsed_path) = parsed.canonicalize() {
                return Some(parsed_path);
            }
        }
    }

    None
}

/// checks for a config specific to the project's root directory
fn get_project_config(params: &InitializeParams) -> Option<TargetConfig> {
    if let Some(mut path) = get_project_root(params) {
        path.push(".asm-lsp.toml");
        if let Ok(config) = std::fs::read_to_string(&path) {
            let path_s = path.display();
            match toml::from_str::<TargetConfig>(&config) {
                Ok(config) => {
                    info!("Parsing asm-lsp project config from file -> {path_s}\n");
                    return Some(config);
                }
                Err(e) => {
                    error!("Failed to parse project config file {path_s} - Error: {e}\n");
                } // if there's an error we fall through to check for a global config in the caller
            }
        }
    }

    None
}

#[must_use]
pub fn instr_filter_targets(instr: &Instruction, config: &TargetConfig) -> Instruction {
    let mut instr = instr.clone();

    let forms = instr
        .forms
        .iter()
        .filter(|form| {
            (form.gas_name.is_some() && config.assemblers.gas)
                || (form.go_name.is_some() && config.assemblers.go)
                || (form.z80_name.is_some() && config.instruction_sets.z80)
        })
        .map(|form| {
            let mut filtered = form.clone();
            // handle cases where gas and go both have names on the same form
            if !config.assemblers.gas {
                filtered.gas_name = None;
            }
            if !config.assemblers.go {
                filtered.go_name = None;
            }
            if !config.assemblers.z80 {
                filtered.z80_name = None;
            }
            filtered
        })
        .collect();

    instr.forms = forms;
    instr
}
