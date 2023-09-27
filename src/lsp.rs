use crate::types::Column;
use crate::{TargetConfig, Instruction};
use lsp_types::{TextDocumentPositionParams, InitializeParams, Url};
use std::fs::File;
use std::io::BufRead;
use log::info;
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

pub fn get_target_config(params: &InitializeParams) -> TargetConfig {
    // first check workspace folders
    let root_path = if let Some(folders) = &params.workspace_folders {
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
        path
    } else {
        None
    };
    
    // if workspace folders weren't set or came up empty, we check the root_uri
    let root_path = if root_path.is_none() {
        if let Some(root_uri) = &params.root_uri {
            if let Ok(path) = root_uri.to_file_path() {
                Some(path)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        root_path
    };

    // if we have a properly configured root path, check for the config file
    if let Some(mut path) = root_path {
        path.push(".asm-lsp");
        if let Ok(config) = std::fs::read_to_string(path) {
            match toml::from_str::<TargetConfig>(&config) {
                Ok(config) => return config,
                Err(e) => {
                    info!(
                        "Failed to parse config file: {e}\n"
                    );
                } // if there's an error we fall through to the default
            }
        }
    }

    // default is to turn everything on
    return TargetConfig::default();
}

pub fn filter_targets(instr: &&Instruction, config: &TargetConfig) -> Instruction {
    let mut instr = (*instr).clone();

    let forms = instr
        .forms
        .iter()
        .filter(|form| {
            (form.go_name.is_some() && config.assemblers.go)
                || (form.gas_name.is_some() && config.assemblers.gas)
        })
    .map(|form| {
        let mut filtered = form.clone();
        // handle cases where gas and go both have names on the same form
        if !(config.assemblers.gas) {
            filtered.gas_name = None;
        }
        if !(config.assemblers.go) {
            filtered.go_name = None;
        }
        filtered
    })
    .collect();

    instr.forms = forms;
    return instr;
}
