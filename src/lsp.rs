use crate::types::Column;
use crate::{Arch, Completable, Hoverable, Instruction, NameToInstructionMap, TargetConfig};
use dirs::config_dir;
use log::{error, info, log, log_enabled};
use lsp_textdocument::FullTextDocument;
use lsp_types::*;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, File};
use std::io::BufRead;
use std::path::PathBuf;
use symbolic::common::{Language, Name, NameMangling};
use symbolic_demangle::{Demangle, DemangleOptions};
use tree_sitter::{InputEdit, Parser, Tree};

/// Find the start and end indices of a word inside the given line
/// Borrowed from RLS
/// `additional_chars` parameter allows specifying additional legal "word"
/// characters besides the default alphanumeric and '_'
pub fn find_word_at_pos(line: &str, col: Column, additional_chars: &str) -> (Column, Column) {
    let line_ = format!("{} ", line);
    let is_ident_char = |c: char| {
        c.is_alphanumeric() || c == '_' || additional_chars.chars().any(|word_char| word_char == c)
    };

    let start = line_
        .chars()
        .enumerate()
        .take(col)
        .filter(|&(_, c)| !is_ident_char(c))
        .last()
        .map(|(i, _)| i + 1)
        .unwrap_or(0);

    #[allow(clippy::filter_next)]
    let mut end = line_
        .chars()
        .enumerate()
        .skip(col)
        .filter(|&(_, c)| !is_ident_char(c));

    let end = end.next();
    (start, end.map(|(i, _)| i).unwrap_or(col))
}

pub fn get_word_from_file_params(
    pos_params: &TextDocumentPositionParams,
    extra_chars: &str,
) -> anyhow::Result<String> {
    let uri = &pos_params.text_document.uri;
    let line = pos_params.position.line as usize;
    let col = pos_params.position.character as usize;

    let filepath = uri.to_file_path();
    match filepath {
        Ok(file) => {
            let file = File::open(file).unwrap_or_else(|_| panic!("Couldn't open file -> {}", uri));
            let buf_reader = std::io::BufReader::new(file);

            let line_conts = buf_reader.lines().nth(line).unwrap().unwrap();
            let (start, end) = find_word_at_pos(&line_conts, col, extra_chars);
            Ok(String::from(&line_conts[start..end]))
        }
        Err(_) => Err(anyhow::anyhow!("filepath get error")),
    }
}

/// Returns a string slice to the word in doc specified by the position params
///
/// The `extra_chars` param allows specifying extra chars to be considered as
/// valid word chars, in addition to the default alphanumeric and '_' chars
// extra_chars is used when grabbing filenames from a document, as '.' isn't
// normally considered a valid "word" char
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

/// return a vector of #include directories
pub fn get_include_dirs() -> Vec<PathBuf> {
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
pub fn text_doc_change_to_ts_edit(
    change: &TextDocumentContentChangeEvent,
    doc: &FullTextDocument,
) -> Result<InputEdit, &'static str> {
    let range = change.range.ok_or("Invalid edit range")?;
    let start = range.start;
    let end = range.end;

    let start_byte = doc.offset_at(start) as usize;
    let new_end_byte = start_byte + change.text.len();
    let new_end_pos = doc.position_at(new_end_byte as u32);

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

/// Given a NameTo_SomeItem_ map, returns a `Vec<CompletionItem>` for the items
/// contained within the map
pub fn get_completes<T: Completable>(
    map: &HashMap<(Arch, &str), T>,
    kind: Option<CompletionItemKind>,
) -> Vec<CompletionItem> {
    map.iter()
        .map(|((_arch, name), item_info)| {
            let value = format!("{}", item_info);

            CompletionItem {
                label: String::from(*name),
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

pub fn get_hover_resp<T: Hoverable, S: Hoverable>(
    word: &str,
    file_word: &str,
    instruction_map: &HashMap<(Arch, &str), T>,
    register_map: &HashMap<(Arch, &str), S>,
    include_dirs: &[PathBuf],
) -> Option<Hover> {
    let instr_lookup = lookup_hover_resp(word, instruction_map);
    if instr_lookup.is_some() {
        return instr_lookup;
    }

    let reg_lookup = lookup_hover_resp(word, register_map);
    if reg_lookup.is_some() {
        return reg_lookup;
    }

    let demang = get_demangle_resp(word);

    if demang.is_some() {
        return demang;
    }

    let include_path = get_include_resp(file_word, include_dirs);
    if include_path.is_some() {
        return include_path;
    }

    None
}

fn lookup_hover_resp<T: Hoverable>(word: &str, map: &HashMap<(Arch, &str), T>) -> Option<Hover> {
    let (x86_res, x86_64_res) = search_for_hoverable(word, map);

    match (x86_res.is_some(), x86_64_res.is_some()) {
        (true, _) | (_, true) => {
            let mut value = String::new();
            if let Some(x86_res) = x86_res {
                value += &format!("{}", x86_res);
            }
            if let Some(x86_64_res) = x86_64_res {
                value += &format!(
                    "{}{}",
                    if x86_res.is_some() { "\n\n" } else { "" },
                    x86_64_res
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

fn get_include_resp(filename: &str, include_dirs: &[PathBuf]) -> Option<Hover> {
    let mut paths = String::new();
    for dir in include_dirs.iter() {
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
    parser: &mut Parser,
    curr_tree: &mut Option<Tree>,
    params: &CompletionParams,
    instr_comps: &[CompletionItem],
    reg_comps: &[CompletionItem],
) -> Option<CompletionList> {
    let cursor_line = params.text_document_position.position.line as usize;
    let cursor_char = params.text_document_position.position.character as usize;

    // prepend register names with "%" in GAS
    if let Some(ctx) = params.context.as_ref() {
        if ctx.trigger_kind == CompletionTriggerKind::TRIGGER_CHARACTER {
            return Some(CompletionList {
                is_incomplete: true,
                items: filtered_comp_list(reg_comps),
            });
        }
    }

    // TODO: filter register completions by width allowed by corresponding instruction
    *curr_tree = parser.parse(curr_doc, curr_tree.as_ref());
    if let Some(tree) = curr_tree {
        let mut cursor = tree_sitter::QueryCursor::new();
        cursor.set_point_range(std::ops::Range {
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

        static QUERY_INSTR_ANY: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(
                tree_sitter_asm::language(),
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

        let matches_iter = cursor.matches(&QUERY_INSTR_ANY, tree.root_node(), curr_doc);
        for match_ in matches_iter {
            let caps = match_.captures;
            for (cap_num, cap) in caps.iter().enumerate() {
                let arg_start = cap.node.range().start_point;
                let arg_end = cap.node.range().end_point;
                if cursor_matches!(cursor_line, cursor_char, arg_start, arg_end) {
                    // an instruction is always capture #0, any capture number after must be a register
                    let items =
                        filtered_comp_list(if cap_num == 0 { instr_comps } else { reg_comps });
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
    parser: &mut tree_sitter::Parser,
    curr_tree: &mut Option<Tree>,
    _params: &DocumentSymbolParams,
) -> Option<Vec<DocumentSymbol>> {
    //let tree = parser.parse(curr_doc, None)?;
    *curr_tree = parser.parse(curr_doc, curr_tree.as_ref());

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
            res.push(doc)
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
    if let Some(tree) = curr_tree {
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
    parser: &mut tree_sitter::Parser,
    params: &SignatureHelpParams,
    curr_tree: &mut Option<Tree>,
    instr_info: &NameToInstructionMap,
) -> Option<SignatureHelp> {
    let cursor_line = params.text_document_position_params.position.line as usize;

    *curr_tree = parser.parse(curr_doc, curr_tree.as_ref());
    if let Some(tree) = curr_tree {
        let mut cursor = tree_sitter::QueryCursor::new();
        cursor.set_point_range(std::ops::Range {
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

        // Instruction with any (or zero) argument(s)
        static QUERY_INSTR_ANY_ARGS: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(
                tree_sitter_asm::language(),
                "(instruction kind: (word) @instr_name)",
            )
            .unwrap()
        });

        let matches: Vec<tree_sitter::QueryMatch<'_, '_>> = cursor
            .matches(&QUERY_INSTR_ANY_ARGS, tree.root_node(), curr_doc)
            .collect();
        if let Some(match_) = matches.first() {
            let caps = match_.captures;
            if caps.len() == 1 {
                if let Ok(instr_name) = caps[0].node.utf8_text(curr_doc) {
                    let mut value = String::new();
                    let mut has_x86 = false;
                    let mut has_x86_64 = false;
                    let (x86_info, x86_64_info) = search_for_hoverable(instr_name, instr_info);
                    if let Some(sig) = x86_info {
                        for form in sig.forms.iter() {
                            if let Some(ref gas_name) = form.gas_name {
                                if instr_name.eq_ignore_ascii_case(gas_name) {
                                    if !has_x86 {
                                        value += "**x86**\n";
                                        has_x86 = true;
                                    }
                                    value += &format!("{}\n", form);
                                }
                            } else if let Some(ref go_name) = form.go_name {
                                if instr_name.eq_ignore_ascii_case(go_name) {
                                    if !has_x86 {
                                        value += "**x86**\n";
                                        has_x86 = true;
                                    }
                                    value += &format!("{}\n", form);
                                }
                            }
                        }
                    }
                    if let Some(sig) = x86_64_info {
                        for form in sig.forms.iter() {
                            if let Some(ref gas_name) = form.gas_name {
                                if instr_name.eq_ignore_ascii_case(gas_name) {
                                    if !has_x86_64 {
                                        value += "**x86_64**\n";
                                        has_x86_64 = true;
                                    }
                                    value += &format!("{}\n", form);
                                }
                            } else if let Some(ref go_name) = form.go_name {
                                if instr_name.eq_ignore_ascii_case(go_name) {
                                    if !has_x86_64 {
                                        value += "**x86_64**\n";
                                        has_x86_64 = true;
                                    }
                                    value += &format!("{}\n", form);
                                }
                            }
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
    parser: &mut Parser,
    curr_tree: &mut Option<Tree>,
    params: &GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let doc = curr_doc.get_content(None);
    *curr_tree = parser.parse(doc, curr_tree.as_ref());

    if let Some(tree) = curr_tree {
        static QUERY_LABEL: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(tree_sitter_asm::language(), "(label) @label").unwrap()
        });

        let is_not_ident_char = |c: char| !(c.is_alphanumeric() || c == '_');
        let mut cursor = tree_sitter::QueryCursor::new();
        let matches = cursor.matches(&QUERY_LABEL, tree.root_node(), doc.as_bytes());

        let word = get_word_from_pos_params(curr_doc, &params.text_document_position_params, "");

        for match_ in matches.into_iter() {
            for cap in match_.captures.iter() {
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
    curr_doc: &FullTextDocument,
    parser: &mut Parser,
    curr_tree: &mut Option<Tree>,
    params: &ReferenceParams,
) -> Vec<Location> {
    let mut refs: Vec<Location> = Vec::new();
    let doc = curr_doc.get_content(None);
    *curr_tree = parser.parse(doc, curr_tree.as_ref());

    if let Some(tree) = curr_tree {
        static QUERY_LABEL: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(
                tree_sitter_asm::language(),
                "(label (ident (reg (word)))) @label",
            )
            .unwrap()
        });

        static QUERY_WORD: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(tree_sitter_asm::language(), "(ident) @ident").unwrap()
        });

        let is_not_ident_char = |c: char| !(c.is_alphanumeric() || c == '_');
        let word = get_word_from_pos_params(curr_doc, &params.text_document_position, "");
        let uri = &params.text_document_position.text_document.uri;

        let mut cursor = tree_sitter::QueryCursor::new();
        if params.context.include_declaration {
            let label_matches = cursor.matches(&QUERY_LABEL, tree.root_node(), doc.as_bytes());
            for match_ in label_matches.into_iter() {
                for cap in match_.captures.iter() {
                    let text = cap
                        .node
                        .utf8_text(doc.as_bytes())
                        .unwrap_or("")
                        .trim()
                        .trim_matches(is_not_ident_char);

                    if word.eq(text) {
                        let start = lsp_pos_of_point(cap.node.start_position());
                        let end = lsp_pos_of_point(cap.node.end_position());
                        // None of the LSP types implement the Hash trait, can't use a HashSet
                        // to avoid duplicates
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
        for match_ in word_matches.into_iter() {
            for cap in match_.captures.iter() {
                let text = cap
                    .node
                    .utf8_text(doc.as_bytes())
                    .unwrap_or("")
                    .trim()
                    .trim_matches(is_not_ident_char);

                if word.eq(text) {
                    let start = lsp_pos_of_point(cap.node.start_position());
                    let end = lsp_pos_of_point(cap.node.end_position());
                    // None of the LSP types implement the Hash trait, can't use a HashSet
                    // to avoid duplicates
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
fn search_for_hoverable<'a, T: Hoverable>(
    word: &'a str,
    map: &'a HashMap<(Arch, &str), T>,
) -> (Option<&'a T>, Option<&'a T>) {
    let x86_res = map.get(&(Arch::X86, word));
    let x86_64_res = map.get(&(Arch::X86_64, word));

    (x86_res, x86_64_res)
}

/// Returns a series of non-overlapping edits from bottom to top
/// Labels are non-indented, non-labels are indented once, and
/// trailing newlines/ whitespace is trimmed according to params
pub fn get_doc_fmt_resp(
    doc: &FullTextDocument,
    params: &DocumentFormattingParams,
) -> Vec<TextEdit> {
    let mut edits: Vec<TextEdit> = Vec::new();

    if doc.line_count() == 0 {
        return edits;
    }

    let single_indent = if params.options.insert_spaces {
        " ".repeat(params.options.tab_size as usize)
    } else {
        String::from("\t")
    };
    const EMPTY_STR: &str = "";
    const NEW_LINE: &str = "\n";

    let insert_final_newline = params.options.insert_final_newline.unwrap_or(false);
    let trim_final_newlines = params.options.trim_final_newlines.unwrap_or(true);
    let trim_trailing_whitespace = params.options.trim_trailing_whitespace.unwrap_or(true);

    // Find the lowest line (highest line number) that isn't a trailing newline
    let mut lowest_nonempty_line = doc.line_count() - 1;
    for line_num in (0..doc.line_count()).rev() {
        lowest_nonempty_line = line_num;
        if !doc
            .get_content(Some(Range {
                start: Position {
                    line: line_num,
                    character: 0,
                },
                end: Position {
                    line: line_num,
                    character: u32::MAX,
                },
            }))
            .trim()
            .is_empty()
        {
            break;
        }
    }

    // handle trimming of/ adding trailing newlines per params
    match (trim_final_newlines, insert_final_newline) {
        // replace all trailing newlines with a single one
        (true, true) => edits.push(TextEdit {
            range: Range {
                start: Position {
                    line: lowest_nonempty_line,
                    character: u32::MAX,
                },
                end: Position {
                    line: doc.line_count() - 1,
                    character: u32::MAX,
                },
            },
            new_text: NEW_LINE.to_string(),
        }),
        // take out all trailing newlines, no additional lines added
        (true, false) => edits.push(TextEdit {
            range: Range {
                start: Position {
                    line: lowest_nonempty_line,
                    character: u32::MAX,
                },
                end: Position {
                    line: doc.line_count() - 1,
                    character: u32::MAX,
                },
            },
            new_text: EMPTY_STR.to_string(),
        }),
        // Add another trailing newline on top of 0 or more existing trailing newlines
        (false, true) => {
            if lowest_nonempty_line < doc.line_count() - 1 {
                edits.push(TextEdit {
                    range: Range {
                        start: Position {
                            line: doc.line_count() - 1,
                            character: u32::MAX,
                        },
                        end: Position {
                            line: doc.line_count() - 1,
                            character: u32::MAX,
                        },
                    },
                    new_text: NEW_LINE.to_string(),
                })
            }
        }
        (false, false) => {} // do nothing
    }

    // Match leading white space if there is any, the label text, ':' literally,
    // white space before a comment, an optional comment, and trailing whitespace
    static LABEL_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?P<lead_ws>^\s*)(?P<label>[a-zA-Z0-9_\.]+):(\s*\S.*\S)*(?P<trail_ws>\s*$)")
            .unwrap()
    });
    // Match leading white space, whatever the contents are (code and comments),
    // and trailing whitespace
    static OTHER_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?P<lead_ws>^\s*)(?P<content>\S.*\S|\S)(?P<trail_ws>\s*$)").unwrap()
    });

    for line_num in (0..=lowest_nonempty_line).rev() {
        let line_contents = doc.get_content(Some(Range {
            start: Position {
                line: line_num,
                character: 0,
            },
            end: Position {
                line: line_num,
                character: u32::MAX,
            },
        }));

        // Case 1: It's a label-> clear any leading whitespace, trim trailing whitespace if
        // specified by params
        if let Some(caps) = LABEL_REGEX.captures(line_contents) {
            if let (Some(lead_ws_match), Some(_label_match), Some(trail_ws_match)) = (
                caps.name("lead_ws"),
                caps.name("label"),
                caps.name("trail_ws"),
            ) {
                {
                    if !lead_ws_match.as_str().is_empty() {
                        edits.push(TextEdit::new(
                            Range {
                                start: Position {
                                    line: line_num,
                                    character: 0,
                                },
                                end: Position {
                                    line: line_num,
                                    character: lead_ws_match.end() as u32,
                                },
                            },
                            EMPTY_STR.to_string(),
                        ));
                    }
                    if trim_trailing_whitespace
                        && !(trail_ws_match.is_empty() || trail_ws_match.as_str().eq("\n"))
                    {
                        edits.push(TextEdit::new(
                            Range {
                                start: Position {
                                    line: line_num,
                                    character: trail_ws_match.start() as u32,
                                },
                                end: Position {
                                    line: line_num,
                                    character: trail_ws_match.end() as u32,
                                },
                            },
                            EMPTY_STR.to_string(),
                        ));
                    }
                }
            }

        // Case 2: It's not a label, make sure leading whitespace is a single indent,
        // trim trailing whitespace if specified by params
        } else if let Some(caps) = OTHER_REGEX.captures(line_contents) {
            if let (Some(lead_ws_match), Some(_content_match), Some(trail_ws_match)) = (
                caps.name("lead_ws"),
                caps.name("content"),
                caps.name("trail_ws"),
            ) {
                if !lead_ws_match.as_str().eq(&single_indent) {
                    edits.push(TextEdit::new(
                        Range {
                            start: Position {
                                line: line_num,
                                character: 0,
                            },
                            end: Position {
                                line: line_num,
                                character: lead_ws_match.end() as u32,
                            },
                        },
                        single_indent.to_string(),
                    ));
                }
                if trim_trailing_whitespace
                    && !(trail_ws_match.is_empty() || trail_ws_match.as_str().eq("\n"))
                {
                    edits.push(TextEdit::new(
                        Range {
                            start: Position {
                                line: line_num,
                                character: trail_ws_match.start() as u32,
                            },
                            end: Position {
                                line: line_num,
                                character: trail_ws_match.end() as u32,
                            },
                        },
                        EMPTY_STR.to_string(),
                    ));
                }
            }
        }
    }

    edits
}

/// Searches for global config in ~/.config/asm-lsp, then the project's directory
/// Project specific configs will override global configs
pub fn get_target_config(params: &InitializeParams) -> TargetConfig {
    match (get_global_config(), get_project_config(params)) {
        (_, Some(proj_cfg)) => proj_cfg,
        (Some(global_cfg), None) => global_cfg,
        (None, None) => TargetConfig::default(), // default is to turn everything on
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

/// checks for a config specific to the current buffer
fn get_project_config(params: &InitializeParams) -> Option<TargetConfig> {
    // 1. if we have workspace folders, then iterate through them and assign the first valid one to
    //    the root path
    // 2. If we don't have worksace folders or none of them is a valid path, check the root_uri
    //    variable
    // 3. If we do have a root_path, check whether we can find a .asm-lsp file at its root.
    let mut root_path: Option<PathBuf> = None;

    // first check workspace folders
    if let Some(folders) = &params.workspace_folders {
        // if there's multiple, just visit in order until we find a valid folder
        let mut path = None;
        for folder in folders.iter() {
            if let Ok(parsed) = Url::parse(folder.uri.as_str()) {
                if let Ok(parsed_path) = parsed.to_file_path() {
                    path = Some(parsed_path);
                    break;
                }
            }
        }

        root_path = path;
    };

    // if workspace folders weren't set or came up empty, we check the root_uri
    if root_path.is_none() {
        if let Some(root_uri) = &params.root_uri {
            if let Ok(path) = root_uri.to_file_path() {
                root_path = Some(path)
            }
        }
    };

    // if we have a properly configured root path, check for the config file
    if let Some(mut path) = root_path {
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
                } // if there's an error we fall through to check for a global config
            }
        }
    }

    None
}

pub fn instr_filter_targets(instr: &Instruction, config: &TargetConfig) -> Instruction {
    let mut instr = instr.clone();

    let forms = instr
        .forms
        .iter()
        .filter(|form| {
            (form.gas_name.is_some() && config.assemblers.gas)
                || (form.go_name.is_some() && config.assemblers.go)
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
            filtered
        })
        .collect();

    instr.forms = forms;
    instr
}
