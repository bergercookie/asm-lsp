use crate::types::Column;
use crate::{Arch, Hoverable, Instruction, TargetConfig};
use log::{error, info};
use lsp_types::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
use std::path::PathBuf;

/// Find the start and end indices of a word inside the given line
/// Borrowed from RLS
pub fn find_word_at_pos(line: &str, col: Column) -> (Column, Column) {
    let line_ = format!("{} ", line);
    let is_ident_char = |c: char| c.is_alphanumeric() || c == '_';

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
            let (start, end) = find_word_at_pos(&line_conts, col);
            Ok(String::from(&line_conts[start..end]))
        }
        Err(_) => Err(anyhow::anyhow!("filepath get error")),
    }
}

pub fn get_hover_resp<T: Hoverable>(word: &str, map: &HashMap<(Arch, &str), T>) -> Option<Hover> {
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

pub fn get_target_config(params: &InitializeParams) -> TargetConfig {
    // 1. if we have workspace folders, then iterate through them and assign the first valid one to
    //    the root path
    // 2. If we don't have worksace folders or none of them is a valid path, check the root_uri
    //    variable
    // 3. If we do have a root_path, check whether we can find a .asm-lsp file at its root.
    // 4. If everything fails return TargetConfig::default()

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
        if let Ok(config) = std::fs::read_to_string(path.clone()) {
            let path_s = path.display();
            match toml::from_str::<TargetConfig>(&config) {
                Ok(config) => {
                    info!("Parsing asm-lsp config from file -> {path_s}\n");
                    return config;
                }
                Err(e) => {
                    error!("Failed to parse config file {path_s} - Error: {e}\n");
                } // if there's an error we fall through to the default
            }
        }
    }

    // default is to turn everything on
    TargetConfig::default()
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
