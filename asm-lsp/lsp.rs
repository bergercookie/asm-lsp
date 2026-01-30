use std::{
    borrow::ToOwned,
    cmp::Ordering,
    collections::{HashMap, HashSet},
    convert::TryFrom as _,
    fmt::Write as _,
    fs::{File, create_dir_all},
    io::BufRead,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr as _,
    string::ToString as _,
    sync::LazyLock,
};

use anyhow::{Result, anyhow};
use compile_commands::{CompilationDatabase, CompileArgs, CompileCommand, SourceFile};
use dirs::config_dir;
use log::{error, info, log, log_enabled, warn};
use lsp_server::{Connection, Message, RequestId, Response};
use lsp_textdocument::FullTextDocument;
use lsp_types::notification::Notification as _;
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, CompletionParams, CompletionTriggerKind,
    Diagnostic, DocumentSymbol, DocumentSymbolParams, Documentation, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverContents, HoverParams, InitializeParams, Location,
    MarkupContent, MarkupKind, MessageType, Position, Range, ReferenceParams, SignatureHelp,
    SignatureHelpParams, SignatureInformation, SymbolKind, TextDocumentContentChangeEvent,
    TextDocumentPositionParams, Uri,
};
use regex::Regex;
use symbolic::common::{Language, Name, NameMangling};
use symbolic_demangle::{Demangle, DemangleOptions};
use tree_sitter::InputEdit;

use crate::{
    Arch, ArchOrAssembler, Assembler, Completable, CompletionItems, Config, ConfigOptions,
    Directive, DocumentStore, Hoverable, Instruction, NameToInstructionMap, RootConfig,
    ServerStore, TreeEntry, types::Column, ustr,
};

/// Prints information about the server
///
/// - Server version
/// - Attempts to find the global configuration directories on the host machine,
///   and indicates whether a `.asm-lsp.toml` config file is present
pub fn run_info() {
    println!("asm-lsp-{}\n", env!("CARGO_PKG_VERSION"));
    let mut global_cfgs: Vec<PathBuf> = get_global_cfg_dirs()
        .iter()
        .filter_map(|p| (*p).clone())
        .collect();
    println!("Default config architecture: {}", Arch::default());
    println!(
        "Global config director{}:",
        if global_cfgs.len() > 1 { "ies" } else { "y" }
    );
    for cfg_path in &mut global_cfgs {
        cfg_path.push("asm-lsp");
        print!("\t{}", cfg_path.display());
        cfg_path.push(".asm-lsp.toml");
        if cfg_path.exists() {
            println!(" -- Config detected");
        } else {
            println!(" -- No config detected");
        }
    }
}

/// Sends an  response indicating no information was available to
/// the lsp client via `connection`
///
/// # Errors
///
/// Returns `Err` if the response fails to send via `connection`
pub fn send_empty_resp(connection: &Connection, id: RequestId) -> Result<()> {
    let empty_resp = Response {
        id,
        result: None,
        error: Some(lsp_server::ResponseError {
            code: lsp_server::ErrorCode::RequestFailed as i32,
            message: "No information available".to_string(),
            data: None,
        }),
    };

    Ok(connection.sender.send(Message::Response(empty_resp))?)
}

/// Sends a notification with to the client
/// Param `message` is the owned type `String` to help reduce redundant allocations
///
/// # Errors
///
/// Returns `Err` if the response fails to send via `connection`
///
/// # Panics
///
/// Panics if JSON encoding of the notification fails
pub fn send_notification(message: String, typ: MessageType, connection: &Connection) -> Result<()> {
    let msg_params = lsp_types::ShowMessageParams { typ, message };
    let result = serde_json::to_value(msg_params).unwrap();
    let err_notif = lsp_server::Notification {
        method: lsp_types::notification::ShowMessage::METHOD.to_string(),
        params: result,
    };
    Ok(connection.sender.send(Message::Notification(err_notif))?)
}

/// Find the ([start], [end]) indices and the cursor's offset in a word
/// on the given line
///
/// Borrowed from RLS
/// characters besides the default alphanumeric and '_'
#[must_use]
pub fn find_word_at_pos(line: &str, col: Column) -> ((Column, Column), usize) {
    let line_ = format!("{line} ");
    // TODO: Let's just pass in a config, this could get messy
    // NOTE: '*' is added as an allowed character to account for the the program
    // counter pseudo variable of the Ca65 assembler. It's included unconditionally
    // here for simplicity, but if this proves to be an issue we can pass in a `config`
    // and only use it if the Ca65 assembler is enabled
    //
    // NOTE: In a similar manner to above, '$' is added as an allowed character to account
    // for mips registers
    let is_ident_char =
        |c: char| c.is_alphanumeric() || c == '_' || c == '.' || c == '*' || c == '$';

    let start = line_
        .chars()
        .enumerate()
        .take(col)
        .filter(|&(_, c)| !is_ident_char(c))
        .last()
        .map_or(0, |(i, _)| i + 1);

    let mut end = line_
        .chars()
        .enumerate()
        .skip(col)
        .filter(|&(_, c)| !is_ident_char(c));

    let end = end.next();
    ((start, end.map_or(col, |(i, _)| i)), col - start)
}

pub enum UriConversion {
    Canonicalized(PathBuf),
    Unchecked(PathBuf),
}

/// Sanitizes the URI path sent by an LSP client
///
/// - "%3A" is replaced by ':' on windows, as this is likely escaped
///   by the emacs client on windows/mingw/msys2
/// - Returning `UriConversion::Canonicalized` indicates a path was able to be
///   canonicalized. This indicates the path is valid and said file exists on disk
/// - Returning `UriConversion::Unchecked` indicates that the path couldn't be
///   canonicalized
///
/// # Panics
///
/// Will panic if `uri` cannot be interpreted as valid utf-8 after being percent-decoded
#[must_use]
pub fn process_uri(uri: &Uri) -> UriConversion {
    let mut clean_path: String =
        url_escape::percent_encoding::percent_decode_str(uri.path().as_str())
            .decode_utf8()
            .unwrap_or_else(|e| {
                panic!(
                    "Invalid encoding for uri \"{}\" -- {e}",
                    uri.path().as_str()
                )
            })
            .to_string();

    // HACK: On Windows, sometimes a leading '/',  e.g. /C:/Users/foo/bar/...
    // is passed as part of the path -- Stuff like Git bash and MSYS2 will accept
    // /C/Users/foo/bar/..., but *not* if the colon is present. Vanila windows
    // will not accept a leading slash at all, but requires the colon after the
    // drive letter, like C:/Users/foo/... So we do our best to clean up here
    if cfg!(windows) && clean_path.contains(':') {
        clean_path = clean_path.strip_prefix('/').unwrap_or(&clean_path).into();
    }

    let Ok(path) = PathBuf::from_str(&clean_path);
    path.canonicalize()
        .map_or(UriConversion::Unchecked(path), |canonicalized| {
            // HACK: On Windows, when a path is canonicalized, sometimes it gets prefixed
            // with "\\?\" -- https://stackoverflow.com/questions/41233684/why-does-my-canonicalized-path-get-prefixed-with
            // That's great and all, but it looks like common tools (like gcc) don't handle
            // this correctly, and you get something like the following:
            // Error: can't open //test.s for reading: No such file or directory
            // The solution? Just cut out the prefix and hope that doesn't break anything else
            if cfg!(windows) {
                #[allow(clippy::option_if_let_else)]
                if let Some(tmp) = canonicalized.to_str().unwrap().strip_prefix("\\\\?\\") {
                    warn!("Stripping Windows canonicalization prefix \"\\\\?\\\" from path");
                    UriConversion::Canonicalized(tmp.into())
                } else {
                    UriConversion::Canonicalized(canonicalized)
                }
            } else {
                UriConversion::Canonicalized(canonicalized)
            }
        })
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
pub fn get_word_from_file_params(pos_params: &TextDocumentPositionParams) -> Result<String> {
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
            let ((start, end), _) = find_word_at_pos(&line_conts, col);
            Ok(String::from(&line_conts[start..end]))
        }
        Err(e) => Err(anyhow!("Filepath get error -- Error: {e}")),
    }
}

/// Returns a string slice to the word in doc specified by the position params,
/// and the cursor's offset into the word
#[must_use]
pub fn get_word_from_pos_params<'a>(
    doc: &'a FullTextDocument,
    pos_params: &TextDocumentPositionParams,
) -> (&'a str, usize) {
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

    let ((word_start, word_end), cursor_offset) =
        find_word_at_pos(line_contents, pos_params.position.character as usize);
    (&line_contents[word_start..word_end], cursor_offset)
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
            .or_insert_with(|| vec![dir.to_owned()]);
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
            && cmd_output.status.success()
        {
            let output_str: String = ustr::get_string(cmd_output.stderr);

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
            match args {
                CompileArgs::Flags(args) | CompileArgs::Arguments(args) => {
                    for arg in args.iter().map(|arg| arg.trim()) {
                        if check_dir {
                            // current arg is preceeded by lone '-I'
                            let dir = PathBuf::from(arg);
                            if dir.is_absolute() {
                                additional_dirs.push((source_file.clone(), dir));
                            } else if let SourceFile::File(ref source_path) = source_file {
                                if let Ok(full_include_path) = source_path.join(dir).canonicalize()
                                {
                                    additional_dirs.push((source_file.clone(), full_include_path));
                                }
                            } else {
                                warn!(
                                    "Additional relative include directories cannot be extracted for a compilation database entry targeting 'All'"
                                );
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
                                if let Ok(full_include_path) = source_path.join(dir).canonicalize()
                                {
                                    additional_dirs.push((source_file.clone(), full_include_path));
                                }
                            } else {
                                warn!(
                                    "Additional relative include directories cannot be extracted for a compilation database entry targeting 'All'"
                                );
                            }
                        }
                    }
                }
            }
        } else if entry.command.is_some()
            && let Some(args) = entry.args_from_cmd()
        {
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

    additional_dirs
}

/// Attempts to find either the `compile_commands.json` or `compile_flags.txt`
/// file in the project's root or build directories, returning either file as a
/// `CompilationDatabase` object
///
/// If both are present, `compile_commands.json` will override `compile_flags.txt`
pub fn get_compile_cmds_from_file(params: &InitializeParams) -> Option<CompilationDatabase> {
    if let Some(mut path) = get_project_root(params) {
        // Check the project root directory first
        let db = get_compilation_db_files(&path);
        if db.is_some() {
            return db;
        }

        // "The convention is to name the file compile_commands.json and put it at the top of the
        // build directory."
        path.push("build");
        let db = get_compilation_db_files(&path);
        if db.is_some() {
            return db;
        }
    }

    None
}

fn get_compilation_db_files(path: &Path) -> Option<CompilationDatabase> {
    // first check for compile_commands.json
    let cmp_cmd_path = path.join("compile_commands.json");
    if let Ok(conts) = std::fs::read_to_string(cmp_cmd_path)
        && let Ok(cmds) = serde_json::from_str(&conts)
    {
        return Some(cmds);
    }
    // then check for compile_flags.txt
    let cmp_flag_path = path.join("compile_flags.txt");
    if let Ok(conts) = std::fs::read_to_string(cmp_flag_path) {
        return Some(compile_commands::from_compile_flags_txt(path, &conts));
    }

    None
}

/// Returns the compile command associated with `uri` if it exists, or the default
/// one from `compile_cmds` otherwise
///
/// - If the user specified a `compiler` *and* flags in their config, use that
/// - If the user specified a `compiler` but no flags in their config (`None`,
///   *not* an empty `Vec`), try to find flags from `compile_flags.txt` in
///   `compile_cmds` and combine the two
/// - If the user didn't specify any compiler info in the relevant config, return
///   the default commands from `compile_cmds`
///
/// # Panics
///
/// Will panic if `req_uri` can't be canonicalized
///
/// NOTE: Several fields within the returned `CompilationDatabase` are intentionally left
/// uninitialized to avoid unnecessary allocations. If you're using this function
/// in a new place, please reconsider this assumption
pub fn get_compile_cmd_for_req(
    config: &RootConfig,
    req_uri: &Uri,
    compile_cmds: &CompilationDatabase,
) -> CompilationDatabase {
    let request_path = match process_uri(req_uri) {
        UriConversion::Canonicalized(p) => p,
        UriConversion::Unchecked(p) => {
            error!(
                "Failed to canonicalize request path {}, using {}",
                req_uri.path().as_str(),
                p.display()
            );
            p
        }
    };
    let config = config.get_config(req_uri);
    match (config.get_compiler(), config.get_compile_flags_txt()) {
        (Some(compiler), Some(flags)) => {
            // Fill out the full command invocation
            let mut args = vec![compiler.to_owned()];
            args.append(&mut flags.clone());
            args.push(request_path.to_str().unwrap_or_default().to_string());
            vec![CompileCommand {
                file: SourceFile::File(request_path),
                directory: PathBuf::new(),
                arguments: Some(CompileArgs::Arguments(args)),
                command: None,
                output: None,
            }]
        }
        (Some(compiler), None) => {
            // Fill out the full command invocation, check if `compile_cmds`
            // has flags to tack on
            let mut args = vec![compiler.to_owned()];
            // Check if we have flags as the first compile command from files,
            // `compile_flags.txt` files get loaded as a single `CompileCommand`
            // object as structured in the below `if` block
            if compile_cmds.len() == 1
                && let CompileCommand {
                    arguments: Some(CompileArgs::Flags(flags)),
                    ..
                } = &compile_cmds[0]
            {
                args.append(&mut flags.clone());
            }
            args.push(request_path.to_str().unwrap_or_default().to_string());
            vec![CompileCommand {
                file: SourceFile::File(request_path),
                directory: PathBuf::new(),
                arguments: Some(CompileArgs::Arguments(args)),
                command: None,
                output: None,
            }]
        }
        (None, Some(flags)) => {
            // Fill out flags, no compiler
            vec![CompileCommand {
                file: SourceFile::File(request_path),
                directory: PathBuf::new(),
                arguments: Some(CompileArgs::Flags(flags.clone())),
                command: None,
                output: None,
            }]
        }
        (None, None) => {
            // Grab the default command from `compile_cmds`
            compile_cmds.clone()
        }
    }
}

/// Returns a default `CompileCommand` for the provided `uri`.
///
/// - If the user specified a compiler in their config, it will be used.
/// - Otherwise, the command will be constructed with a single flag consisting of
///   the provided `uri`
///
/// NOTE: Several fields within the returned `CompileCommand` are intentionally left
/// uninitialized to avoid unnecessary allocations. If you're using this function
/// in a new place, please reconsider this assumption
pub fn get_default_compile_cmd(uri: &Uri, cfg: &Config) -> CompileCommand {
    cfg.get_compiler().as_ref().map_or_else(
        || CompileCommand {
            file: SourceFile::All, // Field isn't checked when called, intentionally left in odd state here
            directory: PathBuf::new(), // Field isn't checked when called, intentionally left uninitialized here
            arguments: Some(CompileArgs::Flags(vec![uri.path().to_string()])),
            command: None,
            output: None,
        },
        |compiler| CompileCommand {
            file: SourceFile::All, // Field isn't checked when called, intentionally left in odd state here
            directory: PathBuf::new(), // Field isn't checked when called, intentionally left uninitialized here
            arguments: Some(CompileArgs::Arguments(vec![
                (*compiler).to_string(),
                uri.path().to_string(),
            ])),
            command: None,
            output: None,
        },
    )
}

/// Attempts to run the given compile command and parses the resulting output. Any
/// relevant output will be translated into a `Diagnostic` object and pushed into
/// `diagnostics`
pub fn apply_compile_cmd(
    cfg: &Config,
    diagnostics: &mut Vec<Diagnostic>,
    uri: &Uri,
    compile_cmd: &CompileCommand,
) {
    info!("Attempting to apply compile command: {compile_cmd:?}");
    // TODO: Consolidate this logic, a little tricky because we need to capture
    // compile_cmd.arguments by reference, but we get an owned Vec out of args_from_cmd()...
    if let Some(ref args) = compile_cmd.arguments {
        match args {
            CompileArgs::Flags(flags) => {
                let compilers = cfg
                    .get_compiler()
                    .as_ref()
                    .map_or_else(|| vec!["gcc", "clang"], |compiler| vec![compiler]);

                for compiler in compilers {
                    match Command::new(compiler) // default or user-supplied compiler
                        .args(flags) // user supplied args
                        .arg(uri.path().as_str()) // the source file in question
                        .output()
                    {
                        Ok(result) => {
                            let output_str = ustr::get_string(result.stderr);
                            get_diagnostics(diagnostics, &output_str, cfg);
                        }
                        Err(e) => {
                            warn!(
                                "Failed to launch compile command process with {compiler} -- Error: {e}"
                            );
                        }
                    }
                }
            }
            CompileArgs::Arguments(arguments) => {
                if arguments.len() < 2 {
                    return;
                }
                let output = match Command::new(&arguments[0]).args(&arguments[1..]).output() {
                    Ok(result) => result,
                    Err(e) => {
                        error!("Failed to launch compile command process -- Error: {e}");
                        return;
                    }
                };
                let output_str = ustr::get_string(output.stderr);
                get_diagnostics(diagnostics, &output_str, cfg);
            }
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
        let output_str = ustr::get_string(output.stderr);
        get_diagnostics(diagnostics, &output_str, cfg);
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
fn get_diagnostics(diagnostics: &mut Vec<Diagnostic>, tool_output: &str, cfg: &Config) {
    // Special handingling for FASM assembler diagnostics
    if cfg.is_assembler_enabled(Assembler::Fasm) {
        // https://flatassembler.net/docs.php?article=manual - 1.1.3 Compile messages
        static FASM_SOURCE_LOC: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^(.+) \[(\d+)\]:$").unwrap());
        // TODO: Look into including macro defintion locations as related information
        // static FASM_MACRO_INSTR_LOC: LazyLock<Regex> =
        //     LazyLock::new(|| Regex::new(r"^(.+) \[(\d+)\]: .+ \[(\d+\)]:$").unwrap());
        static FASM_ERR_MSG: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^error: (.+)").unwrap());

        let mut source_line: Option<u32> = None;
        let mut source_start_col: Option<u32> = None;
        let mut source_end_col: Option<u32> = None;
        let mut lines = tool_output.lines();
        while let Some(line) = lines.next() {
            // for line in tool_output.lines() {
            if let Some(caps) = FASM_SOURCE_LOC.captures(line) {
                // the entire capture is always at the 0th index,
                // then we have 2 more explicit capture groups
                if caps.len() == 3 {
                    let Ok(line_number) = caps[2].parse::<u32>() else {
                        continue;
                    };
                    source_line = Some(line_number);
                    if let Some(src) = lines.next() {
                        let len = src.len() as u32;
                        source_start_col = Some(len - src.trim_start().len() as u32);
                        source_end_col = Some(len);
                    }
                }
            } else if let Some(caps) = FASM_ERR_MSG.captures(line) {
                if caps.len() != 2 {
                    continue;
                }
                if let Some(line_number) = source_line {
                    let err_msg = caps[1].to_string();
                    let start_col = source_start_col.unwrap_or(0);
                    let end_col = source_end_col.unwrap_or(0);
                    diagnostics.push(Diagnostic::new_simple(
                        Range {
                            start: Position {
                                line: line_number - 1,
                                character: start_col,
                            },
                            end: Position {
                                line: line_number - 1,
                                character: end_col,
                            },
                        },
                        err_msg,
                    ));
                }
                source_line = None;
            }
        }
    } else {
        // TODO: Consolidate/ clean this up...regexes are hard
        static DIAG_REG_LINE_COLUMN: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^.*:(\d+):(\d+):\s+(.*)$").unwrap());
        static DIAG_REG_LINE_ONLY: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^.*:(\d+):\s+(.*)$").unwrap());
        static ALT_DIAG_REG_LINE_ONLY: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^.*\((\d+)\):\s+(.*)$").unwrap());
        for line in tool_output.lines() {
            // first check if we have an error message of the form:
            // :<line>:<column>: <error message here>
            if let Some(caps) = DIAG_REG_LINE_COLUMN.captures(line) {
                // the entire capture is always at the 0th index,
                // then we have 3 more explicit capture groups
                if caps.len() == 4 {
                    let Ok(line_number) = caps[1].parse::<u32>() else {
                        continue;
                    };
                    let Ok(column_number) = caps[2].parse::<u32>() else {
                        continue;
                    };
                    let err_msg = &caps[3];
                    diagnostics.push(Diagnostic::new_simple(
                        Range {
                            start: Position {
                                line: line_number - 1,
                                character: column_number,
                            },
                            end: Position {
                                line: line_number - 1,
                                character: column_number,
                            },
                        },
                        String::from(err_msg),
                    ));
                    continue;
                }
            }
            // if the above check for lines *and* columns didn't match, see if we
            // have an error message of the form:
            // :<line>: <error message here>
            if let Some(caps) = DIAG_REG_LINE_ONLY.captures(line) {
                if caps.len() < 3 {
                    // the entire capture is always at the 0th index,
                    // then we have 2 more explicit capture groups
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

            // ca65 has a slightly different format
            // file(<line>): <error message here>
            if let Some(caps) = ALT_DIAG_REG_LINE_ONLY.captures(line) {
                if caps.len() < 3 {
                    // the entire capture is always at the 0th index,
                    // then we have 2 more explicit capture groups
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
}

/// Function allowing us to connect tree sitter's logging with the log crate
#[allow(clippy::needless_pass_by_value)]
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
    let range = change.range.ok_or_else(|| anyhow!("Invalid edit range"))?;
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
    map: &HashMap<(U, String), T>,
    kind: Option<CompletionItemKind>,
) -> Vec<(U, CompletionItem)> {
    map.iter()
        .map(|((arch_or_asm, name), item_info)| {
            let value = item_info.to_string();

            (
                *arch_or_asm,
                CompletionItem {
                    label: (*name).clone(),
                    kind,
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value,
                    })),
                    ..Default::default()
                },
            )
        })
        .collect()
}

#[must_use]
pub fn get_hover_resp(
    params: &HoverParams,
    config: &Config,
    word: &str,
    cursor_offset: usize,
    doc_store: &mut DocumentStore,
    store: &ServerStore,
) -> Option<Hover> {
    let instr_lookup = get_hover_resp_by_arch(word, &store.names_to_info.instructions, config);
    if instr_lookup.is_some() {
        return instr_lookup;
    }

    // directive lookup
    {
        if config.is_assembler_enabled(Assembler::Gas)
            || config.is_assembler_enabled(Assembler::Masm)
            || config.is_assembler_enabled(Assembler::Ca65)
            || config.is_assembler_enabled(Assembler::Avr)
            || config.is_assembler_enabled(Assembler::Fasm)
            || config.is_assembler_enabled(Assembler::Mars)
        {
            // all gas, AVR, and Mars directives have a '.' prefix, some masm directives do
            let directive_lookup =
                get_directive_hover_resp(word, &store.names_to_info.directives, config);
            if directive_lookup.is_some() {
                return directive_lookup;
            }
        } else if config.is_assembler_enabled(Assembler::Nasm) {
            // most nasm directives have no prefix, 2 have a '.' prefix
            let directive_lookup =
                get_directive_hover_resp(word, &store.names_to_info.directives, config);
            if directive_lookup.is_some() {
                return directive_lookup;
            }
            // Some nasm directives have a % prefix
            let prefixed = format!("%{word}");
            let directive_lookup =
                get_directive_hover_resp(&prefixed, &store.names_to_info.directives, config);
            if directive_lookup.is_some() {
                return directive_lookup;
            }
        }
    }

    let reg_lookup = if config.is_isa_enabled(Arch::ARM64) {
        word.find('.').map_or_else(
            || get_hover_resp_by_arch(word, &store.names_to_info.registers, config),
            |dot| {
                if cursor_offset <= dot {
                    // main vector register info on ARM64
                    let main_register = &word[0..dot];
                    get_hover_resp_by_arch(main_register, &store.names_to_info.registers, config)
                } else {
                    // if Vector = V21.2D -> lower Register = D21
                    // lower vector register info on ARM64
                    let reg_len = 3;
                    let mut lower_register = String::with_capacity(reg_len);
                    let reg_letter = dot + 2;
                    lower_register.push_str(&word[reg_letter..]);
                    let reg_num = 1..dot;
                    lower_register.push_str(&word[reg_num]);
                    get_hover_resp_by_arch(&lower_register, &store.names_to_info.registers, config)
                }
            },
        )
    } else {
        get_hover_resp_by_arch(word, &store.names_to_info.registers, config)
    };

    if reg_lookup.is_some() {
        return reg_lookup;
    }

    let label_data = get_label_resp(
        word,
        &params.text_document_position_params.text_document.uri,
        doc_store,
    );
    if label_data.is_some() {
        return label_data;
    }

    let demang = get_demangle_resp(word);
    if demang.is_some() {
        return demang;
    }

    let include_path = get_include_resp(
        &params.text_document_position_params.text_document.uri,
        word,
        &store.include_dirs,
    );
    if include_path.is_some() {
        return include_path;
    }

    None
}

fn search_for_hoverable_by_arch<'a, T: Hoverable>(
    word: &'a str,
    map: &'a HashMap<(Arch, String), T>,
    config: &Config,
) -> (Option<&'a T>, Option<&'a T>) {
    match config.instruction_set {
        Arch::X86_AND_X86_64 => {
            let x86_resp = map.get(&(Arch::X86, word.to_string()));
            let x86_64_resp = map.get(&(Arch::X86_64, word.to_string()));
            (x86_resp, x86_64_resp)
        }
        arch => (map.get(&(arch, word.to_string())), None),
    }
}

fn search_for_dir_by_assembler<'a>(
    word: &'a str,
    dir_map: &'a HashMap<(Assembler, String), Directive>,
    config: &Config,
) -> Option<&'a Directive> {
    dir_map.get(&(config.assembler, word.to_string()))
}

fn get_hover_resp_by_arch<T: Hoverable>(
    word: &str,
    map: &HashMap<(Arch, String), T>,
    config: &Config,
) -> Option<Hover> {
    // ensure hovered text is always lowercase
    let hovered_word = word.to_ascii_lowercase();
    let instr_resp = search_for_hoverable_by_arch(&hovered_word, map, config);
    let value = match instr_resp {
        (Some(resp1), Some(resp2)) => {
            format!("{resp1}\n\n{resp2}")
        }
        (Some(resp), None) | (None, Some(resp)) => {
            format!("{resp}")
        }
        (None, None) => return None,
    };

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: None,
    })
}

fn get_directive_hover_resp(
    word: &str,
    dir_map: &HashMap<(Assembler, String), Directive>,
    config: &Config,
) -> Option<Hover> {
    let hovered_word = word.to_ascii_lowercase();
    search_for_dir_by_assembler(&hovered_word, dir_map, config).map(|dir_resp| Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: dir_resp.to_string(),
        }),
        range: None,
    })
}

/// Returns the data associated with a given label `word`
fn get_label_resp(word: &str, uri: &Uri, doc_store: &mut DocumentStore) -> Option<Hover> {
    if let Some(doc) = doc_store.text_store.get_document(uri) {
        let curr_doc = doc.get_content(None).as_bytes();
        if let Some(ref mut tree_entry) = doc_store.tree_store.get_mut(uri) {
            tree_entry.tree = tree_entry.parser.parse(curr_doc, tree_entry.tree.as_ref());
            if let Some(ref tree) = tree_entry.tree {
                static QUERY_LABEL_DATA: LazyLock<tree_sitter::Query> = LazyLock::new(|| {
                    tree_sitter::Query::new(
                        &tree_sitter_asm::language(),
                        "(
                            (label (ident) @label)
                            .
                            (meta
	                            (
                                    [
                                        (int)
                                        (string)
                                        (float)
                                    ]
                                )
                            ) @data
                        )",
                    )
                    .unwrap()
                });
                let mut cursor = tree_sitter::QueryCursor::new();
                let matches_iter = cursor.matches(&QUERY_LABEL_DATA, tree.root_node(), curr_doc);

                for match_ in matches_iter {
                    let caps = match_.captures;
                    if caps.len() != 2
                        || caps[0].node.end_byte() >= curr_doc.len()
                        || caps[1].node.end_byte() >= curr_doc.len()
                    {
                        continue;
                    }
                    let label_text = caps[0].node.utf8_text(curr_doc);
                    let label_data = caps[1].node.utf8_text(curr_doc);
                    match (label_text, label_data) {
                        (Ok(label), Ok(data))
                            // Some labels have a preceding '.' that we need to account for
                            if label.eq(word) || label.trim_start_matches('.').eq(word) =>
                        {
                            return Some(Hover {
                                contents: HoverContents::Markup(MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value: format!("`{data}`"),
                                }),
                                range: None,
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    None
}

fn get_demangle_resp(word: &str) -> Option<Hover> {
    let name = Name::new(word, NameMangling::Mangled, Language::Unknown);
    let demangled = name.demangle(DemangleOptions::complete());
    if let Some(demang) = demangled {
        let value = demang;
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

    type DirIter<'a> = Box<dyn Iterator<Item = &'a PathBuf> + 'a>;
    let mut dir_iter = include_dirs.get(&SourceFile::All).map_or_else(
        || Box::new(std::iter::empty()) as DirIter,
        |dirs| Box::new(dirs.iter()) as DirIter,
    );

    if let Ok(src_path) = PathBuf::from(source_file.as_str()).canonicalize()
        && let Some(dirs) = include_dirs.get(&SourceFile::File(src_path))
    {
        dir_iter = Box::new(dir_iter.chain(dirs.iter()));
    }

    for dir in dir_iter {
        match std::fs::read_dir(dir) {
            Ok(dir_reader) => {
                for file in dir_reader {
                    match file {
                        Ok(f) => {
                            if f.file_name() == filename {
                                writeln!(&mut paths, "file://{}", f.path().display()).unwrap();
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to read item in {} - Error {e}",
                                dir.as_path().display()
                            );
                        }
                    }
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

/// Filter out duplicate completion suggestions, and those that aren't allowed
/// by `config`
fn filtered_comp_list_arch(
    comps: &[(Arch, CompletionItem)],
    config: &Config,
) -> Vec<CompletionItem> {
    let mut seen = HashSet::new();
    comps
        .iter()
        .filter(|(arch, comp_item)| {
            if !config.is_isa_enabled(*arch) {
                return false;
            }
            if seen.contains(&comp_item.label) {
                false
            } else {
                seen.insert(&comp_item.label);
                true
            }
        })
        .map(|(_, comp_item)| comp_item)
        .cloned()
        .collect()
}

/// Filter out duplicate completion suggestions, and those that aren't allowed
/// by `config`
/// 'prefix' allows the caller to optionally require completion items to start with
/// a given character
fn filtered_comp_list_assem(
    comps: &[(Assembler, CompletionItem)],
    config: &Config,
    prefix: Option<char>,
) -> Vec<CompletionItem> {
    let mut seen = HashSet::new();
    comps
        .iter()
        .filter(|(assem, comp_item)| {
            if !config.is_assembler_enabled(*assem) {
                return false;
            }
            if let Some(c) = prefix
                && !comp_item.label.starts_with(c)
            {
                return false;
            }
            if seen.contains(&comp_item.label) {
                false
            } else {
                seen.insert(&comp_item.label);
                true
            }
        })
        .map(|(_, comp_item)| comp_item)
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
    config: &Config,
    completion_items: &CompletionItems,
) -> Option<CompletionList> {
    let cursor_line = params.text_document_position.position.line as usize;
    let cursor_char = params.text_document_position.position.character as usize;

    if let Some(ctx) = params.context.as_ref()
        && ctx.trigger_kind == CompletionTriggerKind::TRIGGER_CHARACTER
    {
        match ctx
            .trigger_character
            .as_ref()
            .map(std::convert::AsRef::as_ref)
        {
            // prepend GAS registers, some NASM directives with "%"
            Some("%") => {
                let mut items = Vec::new();
                if config.is_isa_enabled(Arch::X86) || config.is_isa_enabled(Arch::X86_64) {
                    items.append(&mut filtered_comp_list_arch(
                        &completion_items.registers,
                        config,
                    ));
                }
                if config.is_assembler_enabled(Assembler::Nasm) {
                    items.append(&mut filtered_comp_list_assem(
                        &completion_items.directives,
                        config,
                        Some('%'),
                    ));
                }

                if !items.is_empty() {
                    return Some(CompletionList {
                        is_incomplete: true,
                        items,
                    });
                }
            }
            // prepend all GAS, all Ca65, all AVR, all Mars, some MASM, some NASM directives with "."
            Some(".") => {
                if config.is_assembler_enabled(Assembler::Gas)
                    || config.is_assembler_enabled(Assembler::Masm)
                    || config.is_assembler_enabled(Assembler::Nasm)
                    || config.is_assembler_enabled(Assembler::Ca65)
                    || config.is_assembler_enabled(Assembler::Avr)
                    || config.is_assembler_enabled(Assembler::Mars)
                {
                    return Some(CompletionList {
                        is_incomplete: true,
                        items: filtered_comp_list_assem(
                            &completion_items.directives,
                            config,
                            Some('.'),
                        ),
                    });
                }
            }
            // prepend all Mips registers with "$"
            Some("$") => {
                if config.is_isa_enabled(Arch::Mips) {
                    return Some(CompletionList {
                        is_incomplete: true,
                        items: filtered_comp_list_arch(&completion_items.registers, config),
                    });
                }
            }
            _ => {}
        }
    }

    // TODO: filter register completions by width allowed by corresponding instruction
    tree_entry.tree = tree_entry.parser.parse(curr_doc, tree_entry.tree.as_ref());
    if let Some(ref tree) = tree_entry.tree {
        static QUERY_DIRECTIVE: LazyLock<tree_sitter::Query> = LazyLock::new(|| {
            tree_sitter::Query::new(
                &tree_sitter_asm::language(),
                "(meta kind: (meta_ident) @directive)",
            )
            .unwrap()
        });
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

        let matches_iter = line_cursor.matches(&QUERY_DIRECTIVE, tree.root_node(), curr_doc);

        for match_ in matches_iter {
            let caps = match_.captures;
            for cap in caps {
                let arg_start = cap.node.range().start_point;
                let arg_end = cap.node.range().end_point;
                if cursor_matches!(cursor_line, cursor_char, arg_start, arg_end) {
                    let items =
                        filtered_comp_list_assem(&completion_items.directives, config, None);
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
        static QUERY_LABEL: LazyLock<tree_sitter::Query> = LazyLock::new(|| {
            tree_sitter::Query::new(&tree_sitter_asm::language(), "(label (ident) @label)").unwrap()
        });

        // need a separate cursor to search the entire document
        let mut doc_cursor = tree_sitter::QueryCursor::new();
        let captures = doc_cursor.captures(&QUERY_LABEL, tree.root_node(), curr_doc);
        let mut labels = HashSet::new();
        for caps in captures.map(|c| c.0) {
            for cap in caps.captures {
                if cap.node.end_byte() >= curr_doc.len() {
                    continue;
                }
                if let Ok(text) = cap.node.utf8_text(curr_doc) {
                    labels.insert(text);
                }
            }
        }

        static QUERY_INSTR_ANY: LazyLock<tree_sitter::Query> = LazyLock::new(|| {
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
                    let mut items = filtered_comp_list_arch(
                        if is_instr {
                            &completion_items.instructions
                        } else {
                            &completion_items.registers
                        },
                        config,
                    );
                    if is_instr {
                        // Sometimes tree-sitter-asm parses a directive as an instruction, so we'll
                        // suggest both in this case
                        items.append(&mut filtered_comp_list_assem(
                            &completion_items.directives,
                            config,
                            None,
                        ));
                    } else {
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

const fn lsp_pos_of_point(pos: tree_sitter::Point) -> lsp_types::Position {
    Position {
        line: pos.row as u32,
        character: pos.column as u32,
    }
}

/// Explore `node`, push immediate children into `res`.
fn explore_node(
    curr_doc: &str,
    node: tree_sitter::Node,
    res: &mut Vec<DocumentSymbol>,
    label_kind_id: u16,
    ident_kind_id: u16,
) {
    if node.kind_id() == label_kind_id {
        let mut children = vec![];
        let mut cursor = node.walk();

        // description for this node
        let mut descr = String::new();

        if cursor.goto_first_child() {
            loop {
                let sub_node = cursor.node();
                if sub_node.kind_id() == ident_kind_id
                    && let Ok(text) = sub_node.utf8_text(curr_doc.as_bytes())
                {
                    descr = text.to_string();
                }

                explore_node(
                    curr_doc,
                    sub_node,
                    &mut children,
                    label_kind_id,
                    ident_kind_id,
                );
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
                explore_node(curr_doc, cursor.node(), res, label_kind_id, ident_kind_id);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
}

/// Get a tree of symbols describing the document's structure.
pub fn get_document_symbols(
    curr_doc: &str,
    tree_entry: &mut TreeEntry,
    _params: &DocumentSymbolParams,
) -> Option<Vec<DocumentSymbol>> {
    static LABEL_KIND_ID: LazyLock<u16> =
        LazyLock::new(|| tree_sitter_asm::language().id_for_node_kind("label", true));
    static IDENT_KIND_ID: LazyLock<u16> =
        LazyLock::new(|| tree_sitter_asm::language().id_for_node_kind("ident", true));
    tree_entry.tree = tree_entry.parser.parse(curr_doc, tree_entry.tree.as_ref());

    tree_entry.tree.as_ref().map(|tree| {
        let mut res: Vec<DocumentSymbol> = vec![];
        let mut cursor = tree.walk();
        loop {
            explore_node(
                curr_doc,
                cursor.node(),
                &mut res,
                *LABEL_KIND_ID,
                *IDENT_KIND_ID,
            );
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        res
    })
}

/// Produces a signature help response if the appropriate instruction forms can
/// be found
pub fn get_sig_help_resp(
    curr_doc: &str,
    params: &SignatureHelpParams,
    config: &Config,
    tree_entry: &mut TreeEntry,
    instr_info: &NameToInstructionMap,
) -> Option<SignatureHelp> {
    let cursor_line = params.text_document_position_params.position.line as usize;

    tree_entry.tree = tree_entry.parser.parse(curr_doc, tree_entry.tree.as_ref());
    if let Some(ref tree) = tree_entry.tree {
        // Instruction with any (including zero) argument(s)
        static QUERY_INSTR_ANY_ARGS: LazyLock<tree_sitter::Query> = LazyLock::new(|| {
            tree_sitter::Query::new(
                &tree_sitter_asm::language(),
                "(instruction kind: (word) @instr_name)",
            )
            .unwrap()
        });

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

        let matches: Vec<tree_sitter::QueryMatch<'_, '_>> = line_cursor
            .matches(&QUERY_INSTR_ANY_ARGS, tree.root_node(), curr_doc)
            .collect();
        if let Some(match_) = matches.first() {
            let caps = match_.captures;
            if caps.len() == 1
                && caps[0].node.end_byte() < curr_doc.len()
                && let Ok(instr_name) = caps[0].node.utf8_text(curr_doc)
            {
                let mut value = String::new();
                let (instr1, instr2) = search_for_hoverable_by_arch(instr_name, instr_info, config);
                let instructions = vec![instr1, instr2];
                for instr in instructions.into_iter().flatten() {
                    for form in &instr.forms {
                        match instr.arch {
                            Arch::X86 | Arch::X86_64 => {
                                if let Some(ref gas_name) = form.gas_name
                                    && instr_name.eq_ignore_ascii_case(gas_name)
                                {
                                    writeln!(&mut value, "**{}**\n{form}", instr.arch).unwrap();
                                } else if let Some(ref go_name) = form.go_name
                                    && instr_name.eq_ignore_ascii_case(go_name)
                                {
                                    writeln!(&mut value, "**{}**\n{form}", instr.arch).unwrap();
                                }
                            }
                            Arch::Z80 => {
                                for form in &instr.forms {
                                    if let Some(ref z80_name) = form.z80_name
                                        && instr_name.eq_ignore_ascii_case(z80_name)
                                    {
                                        writeln!(&mut value, "{form}").unwrap();
                                    }
                                }
                            }
                            Arch::ARM | Arch::RISCV => {
                                for form in &instr.asm_templates {
                                    writeln!(&mut value, "{form}").unwrap();
                                }
                            }
                            _ => {}
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

    None
}

pub fn get_goto_def_resp(
    curr_doc: &FullTextDocument,
    tree_entry: &mut TreeEntry,
    params: &GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let doc = curr_doc.get_content(None).as_bytes();
    tree_entry.tree = tree_entry.parser.parse(doc, tree_entry.tree.as_ref());

    if let Some(ref tree) = tree_entry.tree {
        static QUERY_LABEL: LazyLock<tree_sitter::Query> = LazyLock::new(|| {
            tree_sitter::Query::new(&tree_sitter_asm::language(), "(label) @label").unwrap()
        });

        let is_not_ident_char = |c: char| !(c.is_alphanumeric() || c == '_');
        let mut cursor = tree_sitter::QueryCursor::new();
        let matches = cursor.matches(&QUERY_LABEL, tree.root_node(), doc);

        let (word, _) = get_word_from_pos_params(curr_doc, &params.text_document_position_params);

        for match_ in matches {
            for cap in match_.captures {
                if cap.node.end_byte() >= doc.len() {
                    continue;
                }
                let text = cap
                    .node
                    .utf8_text(doc)
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
    let mut refs: HashSet<Location> = HashSet::new();
    let doc = curr_doc.get_content(None).as_bytes();
    tree_entry.tree = tree_entry.parser.parse(doc, tree_entry.tree.as_ref());

    if let Some(ref tree) = tree_entry.tree {
        static QUERY_LABEL: LazyLock<tree_sitter::Query> = LazyLock::new(|| {
            tree_sitter::Query::new(
                &tree_sitter_asm::language(),
                "(label (ident (reg (word)))) @label",
            )
            .unwrap()
        });

        static QUERY_WORD: LazyLock<tree_sitter::Query> = LazyLock::new(|| {
            tree_sitter::Query::new(&tree_sitter_asm::language(), "(ident) @ident").unwrap()
        });

        let is_not_ident_char = |c: char| !(c.is_alphanumeric() || c == '_');
        let (word, _) = get_word_from_pos_params(curr_doc, &params.text_document_position);
        let uri = &params.text_document_position.text_document.uri;

        let mut cursor = tree_sitter::QueryCursor::new();
        if params.context.include_declaration {
            let label_matches = cursor.matches(&QUERY_LABEL, tree.root_node(), doc);
            for match_ in label_matches {
                for cap in match_.captures {
                    // HACK: Temporary solution for what I believe is a bug in tree-sitter core
                    if cap.node.end_byte() >= doc.len() {
                        continue;
                    }
                    let text = cap
                        .node
                        .utf8_text(doc)
                        .unwrap_or("")
                        .trim()
                        .trim_matches(is_not_ident_char);

                    if word.eq(text) {
                        let start = lsp_pos_of_point(cap.node.start_position());
                        let end = lsp_pos_of_point(cap.node.end_position());
                        refs.insert(Location {
                            uri: uri.clone(),
                            range: Range { start, end },
                        });
                    }
                }
            }
        }

        let word_matches = cursor.matches(&QUERY_WORD, tree.root_node(), doc);
        for match_ in word_matches {
            for cap in match_.captures {
                // HACK: Temporary solution for what I believe is a bug in tree-sitter core
                if cap.node.end_byte() >= doc.len() {
                    continue;
                }
                let text = cap
                    .node
                    .utf8_text(doc)
                    .unwrap_or("")
                    .trim()
                    .trim_matches(is_not_ident_char);

                if word.eq(text) {
                    let start = lsp_pos_of_point(cap.node.start_position());
                    let end = lsp_pos_of_point(cap.node.end_position());
                    refs.insert(Location {
                        uri: uri.clone(),
                        range: Range { start, end },
                    });
                }
            }
        }
    }

    refs.into_iter().collect()
}

/// Searches for global config in ~/.config/asm-lsp, then the project's directory
/// Project specific configs will override global configs
///
/// If no valid config files can be found, this function will cause the program
/// to exit with code -1
///
/// # Errors
///
/// Will return `Err` if an invalid configuration file is found
pub fn get_root_config(params: &InitializeParams) -> Result<RootConfig> {
    let report_err = |msg: &str| -> Result<RootConfig> {
        error!("{msg}");
        Err(anyhow!(msg.to_string()))
    };
    let mut config = match (get_global_config(), get_project_config(params)) {
        // If we have a valid project config, ignore bad global configs
        (_, Some(Ok(proj_cfg))) => proj_cfg,
        (Some(Ok(global_cfg)), None) => global_cfg,
        (Some(Ok(_)) | None, Some(Err(e))) => {
            return report_err(&format!("Inavlid project config file -- {e}"));
        }
        (Some(Err(e)), None) => {
            return report_err(&format!("Invalid global config file -- {e}"));
        }
        (Some(Err(e_global)), Some(Err(e_project))) => {
            return report_err(&format!(
                "Invalid project and global config files -- {e_project} -- {e_global}"
            ));
        }
        (None, None) => {
            info!("No configuration files found, using default options");
            RootConfig::default()
        }
    };

    // Validate project paths and enforce default diagnostics settings
    if let Some(ref mut projects) = config.projects {
        if let Some(ref project_root) = get_project_root(params) {
            let mut project_idx = 0;
            while project_idx < projects.len() {
                let mut project_path = project_root.clone();
                project_path.push(&projects[project_idx].path);
                let Ok(canonicalized_project_path) = project_path.canonicalize() else {
                    return report_err(&format!(
                        "Failed to canonicalize project path \"{}\".",
                        project_path.display()
                    ));
                };
                projects[project_idx].path = canonicalized_project_path;
                if let Some(ref mut opts) = projects[project_idx].config.opts {
                    // Want diagnostics enabled by default
                    if opts.diagnostics.is_none() {
                        opts.diagnostics = Some(true);
                    }
                    // Want default diagnostics enabled by default
                    if opts.default_diagnostics.is_none() {
                        opts.default_diagnostics = Some(true);
                    }
                } else {
                    projects[project_idx].config.opts = Some(ConfigOptions::default());
                }

                project_idx += 1;
            }
        } else {
            return report_err("Unable to detect project root directory.");
        }

        // sort project configurations so when we select a project config at request
        // time, we find configs controlling specific files first, and then configs
        // for a sub-directory of another config before the parent config
        projects.sort_unstable_by(|c1, c2| {
            // - If both are files, we don't care
            // - If one is file and other is directory, file goes first
            // - Else (just assuming both are directories for the default case),
            //   go by the length metric (parent directories get placed *after*
            //   their children)
            let c1_dir = c1.path.is_dir();
            let c1_file = c1.path.is_file();
            let c2_dir = c2.path.is_dir();
            let c2_file = c2.path.is_file();
            if c1_file && c2_file {
                Ordering::Equal
            } else if c1_dir && c2_file {
                Ordering::Greater
            } else if c1_file && c2_dir {
                Ordering::Less
            } else {
                c2.path
                    .to_string_lossy()
                    .len()
                    .cmp(&c1.path.to_string_lossy().len())
            }
        });

        // Check if the user specified multiple configs pointing to the same
        // file or directory
        let mut path_check = HashSet::new();
        for project in projects {
            if path_check.contains(&project.path) {
                return report_err(&format!(
                    "Multiple project configurations for \"{}\".",
                    project.path.display()
                ));
            }
            path_check.insert(&project.path);
        }
    }

    // Enforce default diagnostics settings for default config
    if let Some(ref mut default_cfg) = config.default_config {
        if let Some(ref mut opts) = default_cfg.opts {
            // Want diagnostics enabled by default
            if opts.diagnostics.is_none() {
                opts.diagnostics = Some(true);
            }
            // Want default diagnostics enabled by default
            if opts.default_diagnostics.is_none() {
                opts.default_diagnostics = Some(true);
            }
        } else {
            default_cfg.opts = Some(ConfigOptions::default());
        }
    } else {
        info!("No `default_config` specified, filling in default options");
        // provide a default empty configuration for sub-directories
        // not specified in `projects`
        config.default_config = Some(Config::default());
    }

    Ok(config)
}

#[must_use]
pub fn get_global_cfg_dirs() -> Vec<Option<PathBuf>> {
    if cfg!(target_os = "macos") {
        // `$HOME`/Library/Application Support/ and `$HOME`/.config/
        vec![config_dir(), alt_mac_config_dir()]
    } else {
        vec![config_dir()]
    }
}

/// Checks ~/.config/asm-lsp for a config file, creating directories along the way as necessary
fn get_global_config() -> Option<Result<RootConfig>> {
    let mut paths = get_global_cfg_dirs();

    for cfg_path in paths.iter_mut().flatten() {
        cfg_path.push("asm-lsp");
        info!(
            "Creating directories along {} as necessary...",
            cfg_path.display()
        );
        match create_dir_all(&cfg_path) {
            Ok(()) => {
                cfg_path.push(".asm-lsp.toml");
                if let Ok(config) = std::fs::read_to_string(&cfg_path) {
                    let cfg_path_s = cfg_path.display();
                    match toml::from_str::<RootConfig>(&config) {
                        Ok(config) => {
                            info!(
                                "Parsing global asm-lsp config from file -> {}",
                                cfg_path.display()
                            );
                            return Some(Ok(config));
                        }
                        Err(e) => {
                            error!("Failed to parse global config file {cfg_path_s} - Error: {e}");
                            return Some(Err(e.into()));
                        }
                    }
                }
            }
            Err(e) => {
                error!(
                    "Failed to create global config directory {} - Error: {e}",
                    cfg_path.display()
                );
            }
        }
    }

    None
}

// Returns `$HOME`/.config/ for usage on MacOS, as this isn't the default
// config directory
#[must_use]
pub fn alt_mac_config_dir() -> Option<PathBuf> {
    home::home_dir().map(|mut path| {
        path.push(".config");
        path
    })
}

/// Attempts to find the project's root directory given its `InitializeParams`
// 1. if we have workspace folders, then iterate through them and assign the first valid one to
//    the root path
// 2. If we don't have worksace folders or none of them is a valid path, check the (deprecated)
//    root_uri field
// 3. If both workspace folders and root_uri didn't provide a path, check the (deprecated)
//    root_path field
fn get_project_root(params: &InitializeParams) -> Option<PathBuf> {
    // first check workspace folders
    if let Some(folders) = &params.workspace_folders {
        // if there's multiple, just visit in order until we find a valid folder
        for folder in folders {
            let Ok(parsed) = PathBuf::from_str(folder.uri.path().as_str());
            if let Ok(parsed_path) = parsed.canonicalize() {
                info!("Detected project root: {}", parsed_path.display());
                return Some(parsed_path);
            }
        }
    }

    // if workspace folders weren't set or came up empty, we check the root_uri
    #[allow(deprecated)]
    if let Some(root_uri) = &params.root_uri {
        let Ok(parsed) = PathBuf::from_str(root_uri.path().as_str());
        if let Ok(parsed_path) = parsed.canonicalize() {
            info!("Detected project root: {}", parsed_path.display());
            return Some(parsed_path);
        }
    }

    // if both `workspace_folders` and `root_uri` weren't set or came up empty, we check the root_path
    #[allow(deprecated)]
    if let Some(root_path) = &params.root_path {
        let Ok(parsed) = PathBuf::from_str(root_path.as_str());
        if let Ok(parsed_path) = parsed.canonicalize() {
            return Some(parsed_path);
        }
    }

    warn!("Failed to detect project root");
    None
}

/// checks for a config specific to the project's root directory
///
/// # Errors
///
/// Returns `Err` if the file cannot be deserialized
fn get_project_config(params: &InitializeParams) -> Option<Result<RootConfig>> {
    if let Some(mut path) = get_project_root(params) {
        path.push(".asm-lsp.toml");
        match std::fs::read_to_string(&path) {
            Ok(config) => match toml::from_str::<RootConfig>(&config) {
                Ok(config) => {
                    info!(
                        "Parsing asm-lsp project config from file -> {}",
                        path.display()
                    );
                    return Some(Ok(config));
                }
                Err(e) => {
                    error!(
                        "Failed to parse project config file {} - Error: {e}",
                        path.display()
                    );
                    return Some(Err(e.into()));
                }
            },
            Err(e) => {
                error!("Failed to read config file {} - Error: {e}", path.display());
            }
        }
    }

    None
}

#[must_use]
pub fn instr_filter_targets(instr: &Instruction, config: &Config) -> Instruction {
    let mut instr = instr.clone();

    let forms = instr
        .forms
        .iter()
        .filter(|form| {
            (form.gas_name.is_some() && config.is_assembler_enabled(Assembler::Gas))
                || (form.go_name.is_some() && config.is_assembler_enabled(Assembler::Go))
        })
        .map(|form| {
            let mut filtered = form.clone();
            // handle cases where gas and go both have names on the same form
            if !config.is_assembler_enabled(Assembler::Gas) {
                filtered.gas_name = None;
            }
            if !config.is_assembler_enabled(Assembler::Go) {
                filtered.go_name = None;
            }
            filtered
        })
        .collect();

    instr.forms = forms;
    instr
}
