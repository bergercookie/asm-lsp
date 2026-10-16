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
    Diagnostic, DiagnosticSeverity, DocumentSymbol, DocumentSymbolParams, Documentation,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams,
    InitializeParams, Location, MarkupContent, MarkupKind, MessageType, NumberOrString, Position,
    Range, ReferenceParams, SignatureHelp, SignatureHelpParams, SignatureInformation, SymbolKind,
    TextDocumentContentChangeEvent, TextDocumentPositionParams, Uri,
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
        |c: char| c.is_alphanumeric() || c == '_' || c == '.' || c == '*' || c == '$' || c == '?';

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UriConversion {
    Canonicalized(PathBuf),
    Unchecked(PathBuf),
}

impl UriConversion {
    #[must_use]
    pub fn into_path_buf(self) -> PathBuf {
        match self {
            Self::Canonicalized(p) | Self::Unchecked(p) => p,
        }
    }
}

/// Strips the Windows extended-length path prefix (`\\?\`) if present.
#[must_use]
pub fn strip_windows_prefix(path: PathBuf) -> PathBuf {
    if cfg!(windows)
        && let Some(path_str) = path.to_str()
    {
        if let Some(unc) = path_str.strip_prefix("\\\\?\\UNC\\") {
            return PathBuf::from(format!("\\\\{unc}"));
        }
        if let Some(stripped) = path_str.strip_prefix("\\\\?\\") {
            return PathBuf::from(stripped);
        }
    }
    path
}

/// Canonicalizes `path` and strips the Windows `\\?\` prefix if present.
///
/// # Errors
///
/// Returns `Err` if `std::fs::canonicalize` fails.
pub fn canonicalize_clean(path: &Path) -> std::io::Result<PathBuf> {
    path.canonicalize().map(strip_windows_prefix)
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
    canonicalize_clean(&path).map_or(UriConversion::Unchecked(path), UriConversion::Canonicalized)
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

    let filepath = process_uri(uri).into_path_buf();
    let file = match File::open(&filepath) {
        Ok(opened) => opened,
        Err(e) => return Err(anyhow!("Couldn't open file -> {:?} -- Error: {e}", uri)),
    };
    let buf_reader = std::io::BufReader::new(file);

    let line_conts = buf_reader.lines().nth(line).unwrap().unwrap();
    let ((start, end), _) = find_word_at_pos(&line_conts, col);
    Ok(String::from(&line_conts[start..end]))
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
                .filter_map(|line| canonicalize_clean(Path::new(line.trim())).ok())
                .for_each(|path| {
                    include_dirs.insert(path);
                });

            output_str
                .lines()
                .skip_while(|line| !line.contains("#include <...> search starts here:"))
                .skip(1)
                .take_while(|line| !line.contains("End of search list."))
                .filter_map(|line| canonicalize_clean(Path::new(line.trim())).ok())
                .for_each(|path| {
                    include_dirs.insert(path);
                });
        }
    }

    if let Some(env_include) = std::env::var_os("INCLUDE") {
        for dir in std::env::split_paths(&env_include) {
            if let Ok(canon) = canonicalize_clean(&dir) {
                include_dirs.insert(canon);
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

    let resolve_include_dir = |dir: PathBuf,
                               entry_dir: &Path,
                               source_file: &SourceFile|
     -> Option<PathBuf> {
        if dir.is_absolute() {
            return Some(canonicalize_clean(&dir).unwrap_or(dir));
        }
        if let Ok(full) = canonicalize_clean(&entry_dir.join(&dir)) {
            return Some(full);
        }
        if let SourceFile::File(source_path) = source_file {
            if let Some(parent) = source_path.parent()
                && let Ok(full) = canonicalize_clean(&parent.join(&dir))
            {
                return Some(full);
            }
            if let Ok(full) = canonicalize_clean(&source_path.join(&dir)) {
                return Some(full);
            }
        } else {
            warn!(
                "Additional relative include directories cannot be extracted for a compilation database entry targeting 'All'"
            );
        }
        None
    };

    for entry in compile_cmds {
        let Ok(entry_dir) = canonicalize_clean(&entry.directory) else {
            continue;
        };

        let source_file = match &entry.file {
            SourceFile::All => SourceFile::All,
            SourceFile::File(file) => {
                if file.is_absolute() {
                    SourceFile::File(canonicalize_clean(file).unwrap_or_else(|_| file.clone()))
                } else if let Ok(dir) = canonicalize_clean(&entry_dir.join(file)) {
                    SourceFile::File(dir)
                } else {
                    continue;
                }
            }
        };

        let arg_list: Option<Vec<String>> = match &entry.arguments {
            Some(CompileArgs::Flags(args) | CompileArgs::Arguments(args)) => {
                Some(args.iter().map(|arg| arg.trim().to_string()).collect())
            }
            None => entry.args_from_cmd(),
        };

        if let Some(args) = arg_list {
            let mut check_dir = false;
            for arg in &args {
                if check_dir {
                    if let Some(resolved) =
                        resolve_include_dir(PathBuf::from(arg), &entry_dir, &source_file)
                    {
                        additional_dirs.push((source_file.clone(), resolved));
                    }
                    check_dir = false;
                } else if arg.eq("-I") || arg.eq("/I") {
                    check_dir = true;
                } else if let Some(rest) = arg.strip_prefix("-I").or_else(|| arg.strip_prefix("/I"))
                    && !rest.is_empty()
                    && let Some(resolved) =
                        resolve_include_dir(PathBuf::from(rest), &entry_dir, &source_file)
                {
                    additional_dirs.push((source_file.clone(), resolved));
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

/// Locates `ml64.exe` or `ml.exe` in standard Visual Studio MSVC toolset directories
/// on Windows when the binary is not already on `PATH`.
#[must_use]
fn find_msvc_assembler(tool_name: &str) -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let is_64 =
        tool_name.eq_ignore_ascii_case("ml64") || tool_name.eq_ignore_ascii_case("ml64.exe");
    let exe_name = if is_64 { "ml64.exe" } else { "ml.exe" };
    let target_subpath = if is_64 {
        Path::new("bin").join("Hostx64").join("x64").join(exe_name)
    } else {
        Path::new("bin").join("Hostx64").join("x86").join(exe_name)
    };

    if let Some(vc_tools_dir) = std::env::var_os("VCToolsInstallDir") {
        let candidate = PathBuf::from(vc_tools_dir).join(&target_subpath);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    let roots = [
        std::env::var_os("ProgramFiles").map(PathBuf::from),
        std::env::var_os("ProgramFiles(x86)").map(PathBuf::from),
        Some(PathBuf::from(r"C:\Program Files")),
        Some(PathBuf::from(r"C:\Program Files (x86)")),
    ];

    let mut candidates = Vec::new();
    for root in roots.into_iter().flatten() {
        let vs_root = root.join("Microsoft Visual Studio");
        let Ok(years) = std::fs::read_dir(vs_root) else {
            continue;
        };
        for year in years.flatten() {
            let Ok(editions) = std::fs::read_dir(year.path()) else {
                continue;
            };
            for edition in editions.flatten() {
                let msvc_root = edition.path().join("VC").join("Tools").join("MSVC");
                let Ok(versions) = std::fs::read_dir(msvc_root) else {
                    continue;
                };
                for version in versions.flatten() {
                    let candidate = version.path().join(&target_subpath);
                    if candidate.is_file() {
                        candidates.push(candidate);
                    }
                }
            }
        }
    }
    candidates.sort();
    candidates.pop()
}

fn resolve_compiler_binary(compiler: &str, cfg: &Config) -> String {
    if cfg.is_assembler_enabled(Assembler::Masm)
        && (compiler.eq_ignore_ascii_case("ml64")
            || compiler.eq_ignore_ascii_case("ml64.exe")
            || compiler.eq_ignore_ascii_case("ml")
            || compiler.eq_ignore_ascii_case("ml.exe"))
    {
        if Command::new(compiler).arg("/?").output().is_ok() {
            return compiler.to_string();
        }
        if let Some(found) = find_msvc_assembler(compiler)
            && let Some(path_str) = found.to_str()
        {
            return path_str.to_string();
        }
    }
    compiler.to_string()
}

fn ensure_masm_syntax_check_flags(flags: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(flags.len() + 2);
    let has_syntax_or_compile = flags.iter().any(|f| {
        f.eq_ignore_ascii_case("/Zs")
            || f.eq_ignore_ascii_case("-Zs")
            || f.eq_ignore_ascii_case("/c")
            || f.eq_ignore_ascii_case("-c")
    });
    let has_nologo = flags
        .iter()
        .any(|f| f.eq_ignore_ascii_case("/nologo") || f.eq_ignore_ascii_case("-nologo"));
    if !has_syntax_or_compile {
        out.push("/Zs".to_string());
    }
    if !has_nologo {
        out.push("/nologo".to_string());
    }
    out.extend(flags.iter().cloned());
    out
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
    let source_path = process_uri(uri).into_path_buf();
    let source_arg = source_path
        .to_str()
        .filter(|s| !s.is_empty())
        .map_or_else(|| uri.path().to_string(), ToString::to_string);

    let is_masm = cfg.is_assembler_enabled(Assembler::Masm);
    let arguments = cfg.get_compiler().as_ref().map_or_else(
        || {
            if is_masm {
                CompileArgs::Flags(vec!["/Zs".to_string(), "/nologo".to_string()])
            } else {
                CompileArgs::Flags(vec![source_arg.clone()])
            }
        },
        |compiler| {
            let mut args = vec![(*compiler).to_string()];
            if is_masm {
                args.push("/Zs".to_string());
                args.push("/nologo".to_string());
            }
            args.push(source_arg.clone());
            CompileArgs::Arguments(args)
        },
    );

    CompileCommand {
        file: SourceFile::All, // Field isn't checked when called, intentionally left in odd state here
        directory: PathBuf::new(), // Field isn't checked when called, intentionally left uninitialized here
        arguments: Some(arguments),
        command: None,
        output: None,
    }
}

fn collect_command_output(output: &std::process::Output, cfg: &Config) -> String {
    let stderr_str = ustr::get_string(output.stderr.clone());
    if cfg.is_assembler_enabled(Assembler::Masm) || stderr_str.trim().is_empty() {
        let stdout_str = ustr::get_string(output.stdout.clone());
        if stderr_str.trim().is_empty() {
            stdout_str
        } else if stdout_str.trim().is_empty() {
            stderr_str
        } else {
            format!("{stdout_str}\n{stderr_str}")
        }
    } else {
        stderr_str
    }
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
    let source_path = process_uri(uri).into_path_buf();
    let source_arg = source_path
        .to_str()
        .filter(|s| !s.is_empty())
        .map_or_else(|| uri.path().as_str(), |s| s);
    let working_dir =
        if !compile_cmd.directory.as_os_str().is_empty() && compile_cmd.directory.is_dir() {
            Some(compile_cmd.directory.clone())
        } else {
            source_path
                .parent()
                .filter(|p| !p.as_os_str().is_empty() && p.is_dir())
                .map(Path::to_path_buf)
        };

    let mut run_compiler =
        |compiler: &str, raw_flags: &[String], append_source: Option<&str>| -> bool {
            let resolved = resolve_compiler_binary(compiler, cfg);
            let effective_flags = if cfg.is_assembler_enabled(Assembler::Masm) {
                ensure_masm_syntax_check_flags(raw_flags)
            } else {
                raw_flags.to_vec()
            };
            let mut cmd = Command::new(&resolved);
            if let Some(ref dir) = working_dir {
                cmd.current_dir(dir);
            }
            cmd.args(&effective_flags);
            if let Some(src) = append_source
                && !effective_flags.iter().any(|a| a == src)
            {
                cmd.arg(src);
            }
            match cmd.output() {
                Ok(result) => {
                    let output_str = collect_command_output(&result, cfg);
                    get_diagnostics(diagnostics, &output_str, cfg);
                    true
                }
                Err(e) => {
                    warn!("Failed to launch compile command process with {compiler} -- Error: {e}");
                    false
                }
            }
        };

    if let Some(ref args) = compile_cmd.arguments {
        match args {
            CompileArgs::Flags(flags) => {
                let compilers: Vec<&str> = cfg.get_compiler().as_ref().map_or_else(
                    || {
                        if cfg.is_assembler_enabled(Assembler::Masm) {
                            if cfg.instruction_set == Arch::X86 {
                                vec!["ml", "ml64"]
                            } else {
                                vec!["ml64", "ml"]
                            }
                        } else {
                            vec!["gcc", "clang"]
                        }
                    },
                    |compiler| vec![compiler],
                );

                for compiler in compilers {
                    if run_compiler(compiler, flags, Some(source_arg))
                        && cfg.is_assembler_enabled(Assembler::Masm)
                    {
                        break;
                    }
                }
            }
            CompileArgs::Arguments(arguments) => {
                if arguments.len() >= 2 {
                    run_compiler(&arguments[0], &arguments[1..], None);
                }
            }
        }
    } else if let Some(args) = compile_cmd.args_from_cmd()
        && args.len() >= 2
    {
        run_compiler(&args[0], &args[1..], None);
    }

    if let Ok(source_text) = std::fs::read_to_string(&source_path) {
        refine_diagnostic_ranges(diagnostics, &source_text);
    }
}

/// Refines line-only diagnostic ranges (`character: 0..0`) against `source_text`.
pub fn refine_diagnostic_ranges(diagnostics: &mut [Diagnostic], source_text: &str) {
    let lines: Vec<&str> = source_text.lines().collect();
    for diag in diagnostics {
        if diag.range.start.line != diag.range.end.line
            || diag.range.start.character != 0
            || diag.range.end.character != 0
        {
            continue;
        }
        let line_num = diag.range.start.line;
        let Some(&raw_line) = lines.get(line_num as usize) else {
            continue;
        };
        let code_part = raw_line.split(';').next().unwrap_or(raw_line).trim_end();
        let trimmed = code_part.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        let stmt_start = code_part.len() - trimmed.len();
        let stmt_end = code_part.len();

        let (start_col, end_col) = diag
            .message
            .rsplit_once(':')
            .map(|(_, tok)| tok.split("(in macro").next().unwrap_or(tok).trim())
            .filter(|tok| {
                !tok.is_empty() && !tok.contains(char::is_whitespace) && tok.len() < trimmed.len()
            })
            .and_then(|tok| {
                trimmed
                    .match_indices(tok)
                    .find(|&(idx, _)| {
                        let before_ok = idx == 0
                            || !trimmed[..idx]
                                .chars()
                                .next_back()
                                .is_some_and(|c| c.is_alphanumeric() || c == '_');
                        let after_idx = idx + tok.len();
                        let after_ok = after_idx >= trimmed.len()
                            || !trimmed[after_idx..]
                                .chars()
                                .next()
                                .is_some_and(|c| c.is_alphanumeric() || c == '_');
                        before_ok && after_ok
                    })
                    .map(|(idx, _)| (stmt_start + idx, stmt_start + idx + tok.len()))
            })
            .unwrap_or((stmt_start, stmt_end));

        diag.range = Range {
            start: Position {
                line: line_num,
                character: start_col as u32,
            },
            end: Position {
                line: line_num,
                character: end_col as u32,
            },
        };
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
pub fn get_diagnostics(diagnostics: &mut Vec<Diagnostic>, tool_output: &str, cfg: &Config) {
    if cfg.is_assembler_enabled(Assembler::Masm) {
        static MASM_DIAG: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"(?i)^(.+?)\((\d+)\)\s*:\s*(fatal\s+error|error|warning)\s+([A-Z]\d+)\s*:\s*(.+)$",
            )
            .unwrap()
        });
        static MASM_MACRO_CALL: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\s+([A-Za-z0-9_@$?]+)\(\d+\):\s*Macro Called From\s*$").unwrap()
        });
        static MASM_MAIN_LINE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\s+(.+?)\((\d+)\):\s*(?:Main Line Code|Macro Called From)\s*$").unwrap()
        });

        let mut lines = tool_output.lines().peekable();
        while let Some(line) = lines.next() {
            if let Some(caps) = MASM_DIAG.captures(line) {
                let Ok(mut line_number) = caps[2].parse::<u32>() else {
                    continue;
                };
                let sev_str = caps[3].to_ascii_lowercase();
                let severity = if sev_str.contains("warning") {
                    DiagnosticSeverity::WARNING
                } else {
                    DiagnosticSeverity::ERROR
                };
                let code = caps[4].to_ascii_uppercase();
                let mut msg = caps[5].trim().to_string();

                let mut active_macro: Option<String> = None;
                while let Some(&next_line) = lines.peek() {
                    if let Some(mac_caps) = MASM_MACRO_CALL.captures(next_line) {
                        active_macro = Some(mac_caps[1].to_string());
                        lines.next();
                    } else if let Some(main_caps) = MASM_MAIN_LINE.captures(next_line) {
                        if let Ok(caller_line) = main_caps[2].parse::<u32>() {
                            line_number = caller_line;
                        }
                        if let Some(ref mac_name) = active_macro {
                            let _ = write!(msg, " (in macro `{mac_name}`)");
                            active_macro = None;
                        }
                        lines.next();
                    } else {
                        break;
                    }
                }

                let diag_line = line_number.saturating_sub(1);
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: diag_line,
                            character: 0,
                        },
                        end: Position {
                            line: diag_line,
                            character: 0,
                        },
                    },
                    severity: Some(severity),
                    code: Some(NumberOrString::String(code)),
                    source: Some("masm".to_string()),
                    message: msg,
                    ..Default::default()
                });
            }
        }
        return;
    }

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
            if config.is_assembler_enabled(Assembler::Masm) {
                let uri = &params.text_document_position_params.text_document.uri;
                let line_idx = params.text_document_position_params.position.line as usize;
                let col = params.text_document_position_params.position.character as usize;
                let start_col = col.saturating_sub(cursor_offset);
                let has_at_prefix = doc_store
                    .text_store
                    .get_document(uri)
                    .and_then(|d| d.get_content(None).lines().nth(line_idx))
                    .is_some_and(|l| {
                        start_col > 0 && l.as_bytes().get(start_col - 1) == Some(&b'@')
                    });
                if has_at_prefix {
                    let at_word = format!("@{word}");
                    let lookup =
                        get_directive_hover_resp(&at_word, &store.names_to_info.directives, config);
                    if lookup.is_some() {
                        return lookup;
                    }
                }
            }
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

fn is_masm_data_or_def_kw(kw: &str) -> bool {
    matches!(
        kw.to_ascii_uppercase().as_str(),
        "PROC"
            | "PROTO"
            | "STRUCT"
            | "STRUC"
            | "UNION"
            | "RECORD"
            | "TYPEDEF"
            | "MACRO"
            | "EQU"
            | "TEXTEQU"
            | "="
            | "LABEL"
            | "DB"
            | "DW"
            | "DD"
            | "DF"
            | "DQ"
            | "DT"
            | "DO"
            | "DY"
            | "DZ"
            | "RESB"
            | "RESW"
            | "RESD"
            | "RESQ"
            | "REST"
            | "RESO"
            | "RESY"
            | "RESZ"
            | "BYTE"
            | "SBYTE"
            | "WORD"
            | "SWORD"
            | "DWORD"
            | "SDWORD"
            | "FWORD"
            | "QWORD"
            | "SQWORD"
            | "MMWORD"
            | "TBYTE"
            | "TWORD"
            | "OWORD"
            | "XMMWORD"
            | "YWORD"
            | "YMMWORD"
            | "ZWORD"
            | "ZMMWORD"
            | "REAL4"
            | "REAL8"
            | "REAL10"
    )
}

fn line_token_range(raw_line: &str, line_num: u32, token: &str) -> Range {
    let col = raw_line.find(token).unwrap_or(0);
    Range {
        start: Position {
            line: line_num,
            character: col as u32,
        },
        end: Position {
            line: line_num,
            character: (col + token.len()) as u32,
        },
    }
}

/// Extracts colonless MASM symbol definitions (`PROC`, `PROTO`, `EXTERN`, `LOCAL`,
/// `STRUCT`/`UNION`/`RECORD`/`TYPEDEF`/`MACRO`, `EQU`/`=`, and data declarations).
fn extract_masm_symbols(doc: &str) -> Vec<(String, SymbolKind, String, Range)> {
    let mut out = Vec::new();
    for (line_idx, raw_line) in doc.lines().enumerate() {
        let code = raw_line.split(';').next().unwrap_or("").trim();
        if code.is_empty() {
            continue;
        }
        let line_num = line_idx as u32;
        let mut words = code.split_whitespace();
        let Some(first) = words.next() else {
            continue;
        };
        let first_upper = first.to_ascii_uppercase();

        if matches!(
            first_upper.as_str(),
            "EXTERN" | "EXTRN" | "EXTERNDEF" | "LOCAL"
        ) {
            let kind = if first_upper == "LOCAL" {
                SymbolKind::VARIABLE
            } else {
                SymbolKind::INTERFACE
            };
            for item in code[first.len()..].trim().split(',') {
                let sym = item
                    .split([':', '['])
                    .next()
                    .unwrap_or("")
                    .split_whitespace()
                    .last()
                    .unwrap_or("")
                    .trim();
                if !sym.is_empty() && sym.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    out.push((
                        sym.to_string(),
                        kind,
                        code.to_string(),
                        line_token_range(raw_line, line_num, sym),
                    ));
                }
            }
            continue;
        }

        let name = first.trim_end_matches(':');
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let Some(second) = words.next() else {
            continue;
        };
        if !is_masm_data_or_def_kw(second) {
            continue;
        }
        let kind = match second.to_ascii_uppercase().as_str() {
            "PROC" | "MACRO" => SymbolKind::FUNCTION,
            "PROTO" => SymbolKind::INTERFACE,
            "STRUCT" | "STRUC" | "UNION" | "RECORD" => SymbolKind::STRUCT,
            "TYPEDEF" => SymbolKind::TYPE_PARAMETER,
            "EQU" | "TEXTEQU" | "=" => SymbolKind::CONSTANT,
            _ => SymbolKind::VARIABLE,
        };
        out.push((
            name.to_string(),
            kind,
            code.to_string(),
            line_token_range(raw_line, line_num, name),
        ));
    }
    out
}

/// Returns the data associated with a given label `word`
fn get_label_resp(word: &str, uri: &Uri, doc_store: &mut DocumentStore) -> Option<Hover> {
    if let Some(doc) = doc_store.text_store.get_document(uri) {
        let doc_str = doc.get_content(None);
        let curr_doc = doc_str.as_bytes();
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
        for (sym_name, _, detail, _) in extract_masm_symbols(doc_str) {
            if sym_name.eq_ignore_ascii_case(word) {
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("`{detail}`"),
                    }),
                    range: None,
                });
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
    let filename = filename.trim_matches(|c| c == '"' || c == '\'' || c == '<' || c == '>');
    if filename.is_empty() {
        return None;
    }

    let mut search_dirs: Vec<PathBuf> = Vec::new();
    let src_path = process_uri(source_file).into_path_buf();
    if let Some(parent) = src_path.parent()
        && !parent.as_os_str().is_empty()
    {
        search_dirs.push(parent.to_path_buf());
    }
    if let Some(dirs) = include_dirs.get(&SourceFile::All) {
        search_dirs.extend(dirs.iter().cloned());
    }
    if let Some(dirs) = include_dirs.get(&SourceFile::File(src_path)) {
        search_dirs.extend(dirs.iter().cloned());
    }

    let mut seen = HashSet::new();
    for dir in search_dirs {
        let candidate = dir.join(filename);
        if candidate.is_file() {
            let full = canonicalize_clean(&candidate).unwrap_or(candidate);
            if seen.insert(full.clone()) {
                writeln!(&mut paths, "file://{}", full.display()).unwrap();
            }
            continue;
        }
        match std::fs::read_dir(&dir) {
            Ok(dir_reader) => {
                for file in dir_reader {
                    match file {
                        Ok(f) => {
                            if f.file_name().eq_ignore_ascii_case(filename) {
                                let full =
                                    canonicalize_clean(&f.path()).unwrap_or_else(|_| f.path());
                                if seen.insert(full.clone()) {
                                    writeln!(&mut paths, "file://{}", full.display()).unwrap();
                                }
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
            Some(".")
                if config.is_assembler_enabled(Assembler::Gas)
                    || config.is_assembler_enabled(Assembler::Masm)
                    || config.is_assembler_enabled(Assembler::Nasm)
                    || config.is_assembler_enabled(Assembler::Ca65)
                    || config.is_assembler_enabled(Assembler::Avr)
                    || config.is_assembler_enabled(Assembler::Mars) =>
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
            // prepend all Mips registers with "$"
            Some("$") if config.is_isa_enabled(Arch::Mips) => {
                return Some(CompletionList {
                    is_incomplete: true,
                    items: filtered_comp_list_arch(&completion_items.registers, config),
                });
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
        let masm_syms = if config.is_assembler_enabled(Assembler::Masm) {
            extract_masm_symbols(curr_doc)
        } else {
            Vec::new()
        };
        let curr_doc_str = curr_doc;
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
        let mut labels: HashSet<&str> = HashSet::new();
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
        for (sym_name, _, _, _) in &masm_syms {
            labels.insert(sym_name.as_str());
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

        if config.is_assembler_enabled(Assembler::Masm)
            && let Some(line) = curr_doc_str.lines().nth(cursor_line)
        {
            let before = line
                .get(..cursor_char.min(line.len()))
                .unwrap_or("")
                .split(';')
                .next()
                .unwrap_or("")
                .trim_start();
            if before.contains(char::is_whitespace) {
                let mut items = filtered_comp_list_arch(&completion_items.registers, config);
                items.extend(labels.iter().map(|l| CompletionItem {
                    label: (*l).to_string(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    ..Default::default()
                }));
                if !items.is_empty() {
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
        for (name, kind, detail, range) in extract_masm_symbols(curr_doc) {
            if !res.iter().any(|s| s.name == name) {
                #[allow(deprecated)]
                res.push(DocumentSymbol {
                    name,
                    detail: Some(detail),
                    kind,
                    tags: None,
                    deprecated: Some(false),
                    range,
                    selection_range: range,
                    children: None,
                });
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
                                if config.is_assembler_enabled(Assembler::Gas)
                                    && let Some(ref gas_name) = form.gas_name
                                    && instr_name.eq_ignore_ascii_case(gas_name)
                                {
                                    writeln!(&mut value, "**{}**\n{form}", instr.arch).unwrap();
                                } else if config.is_assembler_enabled(Assembler::Go)
                                    && let Some(ref go_name) = form.go_name
                                    && instr_name.eq_ignore_ascii_case(go_name)
                                {
                                    writeln!(&mut value, "**{}**\n{form}", instr.arch).unwrap();
                                } else if !config.is_assembler_enabled(Assembler::Gas)
                                    && !config.is_assembler_enabled(Assembler::Go)
                                    && (instr.name.eq_ignore_ascii_case(instr_name)
                                        || form
                                            .gas_name
                                            .as_ref()
                                            .is_some_and(|n| instr_name.eq_ignore_ascii_case(n)))
                                {
                                    writeln!(
                                        &mut value,
                                        "**{}**\n{}",
                                        instr.arch,
                                        instr.format_form(form)
                                    )
                                    .unwrap();
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

        for (sym_name, _, _, range) in extract_masm_symbols(curr_doc.get_content(None)) {
            if sym_name.eq_ignore_ascii_case(word) {
                return Some(GotoDefinitionResponse::Scalar(Location {
                    uri: params
                        .text_document_position_params
                        .text_document
                        .uri
                        .clone(),
                    range,
                }));
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
            tree_sitter::Query::new(&tree_sitter_asm::language(), "[(ident) (word)] @ident")
                .unwrap()
        });

        let is_not_ident_char = |c: char| !(c.is_alphanumeric() || c == '_');
        let (word, _) = get_word_from_pos_params(curr_doc, &params.text_document_position);
        let uri = &params.text_document_position.text_document.uri;
        let masm_decl_ranges: HashSet<Range> = extract_masm_symbols(curr_doc.get_content(None))
            .into_iter()
            .filter(|(sym_name, _, _, _)| sym_name.eq(word))
            .map(|(_, _, _, range)| range)
            .collect();

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
            for &range in &masm_decl_ranges {
                refs.insert(Location {
                    uri: uri.clone(),
                    range,
                });
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
                    let range = Range { start, end };
                    if params.context.include_declaration || !masm_decl_ranges.contains(&range) {
                        refs.insert(Location {
                            uri: uri.clone(),
                            range,
                        });
                    }
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
                let Ok(canonicalized_project_path) = canonicalize_clean(&project_path) else {
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
    if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
        // `$HOME`/Library/Application Support/ (or `%APPDATA%`) and `$HOME`/.config/
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
            if let UriConversion::Canonicalized(parsed_path) = process_uri(&folder.uri) {
                info!("Detected project root: {}", parsed_path.display());
                return Some(parsed_path);
            }
        }
    }

    // if workspace folders weren't set or came up empty, we check the root_uri
    #[allow(deprecated)]
    if let Some(root_uri) = &params.root_uri
        && let UriConversion::Canonicalized(parsed_path) = process_uri(root_uri)
    {
        info!("Detected project root: {}", parsed_path.display());
        return Some(parsed_path);
    }

    // if both `workspace_folders` and `root_uri` weren't set or came up empty, we check the root_path
    #[allow(deprecated)]
    if let Some(root_path) = &params.root_path {
        let Ok(parsed) = PathBuf::from_str(root_path.as_str());
        if let Ok(parsed_path) = canonicalize_clean(&parsed) {
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
    if let Some(root) = get_project_root(params) {
        for ancestor in root.ancestors() {
            let path = ancestor.join(".asm-lsp.toml");
            if !path.is_file() {
                continue;
            }
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
                || (!config.is_assembler_enabled(Assembler::Gas)
                    && !config.is_assembler_enabled(Assembler::Go))
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
