use crate::types::Column;
use crate::{Arch, Completable, Hoverable, Instruction, TargetConfig};
use log::{error, info, log, log_enabled};
use lsp_textdocument::FullTextDocument;
use lsp_types::*;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufRead;
use std::path::PathBuf;
use tree_sitter::{InputEdit, Parser, Tree};

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
    let mut comp_items = None;

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

        // Instruction and two register arguments
        static QUERY_INSTR_REG_REG: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(
                tree_sitter_asm::language(),
                "(instruction kind: (word) @instr_name (ident (reg) @r1) (ident (reg) @r2))",
            )
            .unwrap()
        });

        let matches: Vec<tree_sitter::QueryMatch<'_, '_>> = cursor
            .matches(&QUERY_INSTR_REG_REG, tree.root_node(), curr_doc)
            .collect();
        if let Some(match_) = matches.first() {
            let caps = match_.captures;
            if caps.len() == 3 {
                let instr_start = caps[0].node.range().start_point;
                let instr_end = caps[0].node.range().end_point;
                let reg_1_start = caps[1].node.range().start_point;
                let reg_1_end = caps[1].node.range().end_point;
                let reg_2_start = caps[2].node.range().start_point;
                let reg_2_end = caps[2].node.range().end_point;
                if cursor_matches!(cursor_line, cursor_char, instr_start, instr_end) {
                    comp_items = Some(filtered_comp_list(instr_comps));
                } else if cursor_matches!(cursor_line, cursor_char, reg_1_start, reg_1_end)
                    || cursor_matches!(cursor_line, cursor_char, reg_2_start, reg_2_end)
                {
                    comp_items = Some(filtered_comp_list(reg_comps));
                }
                if let Some(items) = comp_items {
                    return Some(CompletionList {
                        is_incomplete: true,
                        items,
                    });
                }
            }
        }

        // Instruction and one register argument, one non-register argument
        static QUERY_INSTR_REG_ARG: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(
                tree_sitter_asm::language(),
                "(instruction kind: (word) @instr_name (ident (reg) @r1) (ident))",
            )
            .unwrap()
        });

        let matches: Vec<tree_sitter::QueryMatch<'_, '_>> = cursor
            .matches(&QUERY_INSTR_REG_ARG, tree.root_node(), curr_doc)
            .collect();
        if let Some(match_) = matches.first() {
            let caps = match_.captures;
            if caps.len() == 2 {
                let instr_start = caps[0].node.range().start_point;
                let instr_end = caps[0].node.range().end_point;
                let reg_start = caps[1].node.range().start_point;
                let reg_end = caps[1].node.range().end_point;
                if cursor_matches!(cursor_line, cursor_char, instr_start, instr_end) {
                    comp_items = Some(filtered_comp_list(instr_comps));
                } else if cursor_matches!(cursor_line, cursor_char, reg_start, reg_end) {
                    comp_items = Some(filtered_comp_list(reg_comps));
                }
                if let Some(items) = comp_items {
                    return Some(CompletionList {
                        is_incomplete: true,
                        items,
                    });
                }
            }
        }

        // Instruction and one non-register argument, one register argument
        static QUERY_INSTR_ARG_REG: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(
                tree_sitter_asm::language(),
                "(instruction kind: (word) @instr_name (ident) (ident (reg) @r1))",
            )
            .unwrap()
        });
        let matches: Vec<tree_sitter::QueryMatch<'_, '_>> = cursor
            .matches(&QUERY_INSTR_ARG_REG, tree.root_node(), curr_doc)
            .collect();
        if let Some(match_) = matches.first() {
            let caps = match_.captures;
            if caps.len() == 2 {
                let instr_start = caps[0].node.range().start_point;
                let instr_end = caps[0].node.range().end_point;
                let reg_start = caps[1].node.range().start_point;
                let reg_end = caps[1].node.range().end_point;
                if cursor_matches!(cursor_line, cursor_char, instr_start, instr_end) {
                    comp_items = Some(filtered_comp_list(instr_comps));
                } else if cursor_matches!(cursor_line, cursor_char, reg_start, reg_end) {
                    comp_items = Some(filtered_comp_list(reg_comps));
                }
                if let Some(items) = comp_items {
                    return Some(CompletionList {
                        is_incomplete: true,
                        items,
                    });
                }
            }
        }

        // Instruction and one register argument
        static QUERY_INSTR_REG: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(
                tree_sitter_asm::language(),
                "(instruction kind: (word) @instr_name (ident (reg) @r1))",
            )
            .unwrap()
        });
        let matches: Vec<tree_sitter::QueryMatch<'_, '_>> = cursor
            .matches(&QUERY_INSTR_REG, tree.root_node(), curr_doc)
            .collect();
        if let Some(match_) = matches.first() {
            let caps = match_.captures;
            if caps.len() == 2 {
                let instr_start = caps[0].node.range().start_point;
                let instr_end = caps[0].node.range().end_point;
                let reg_start = caps[1].node.range().start_point;
                let reg_end = caps[1].node.range().end_point;
                if cursor_matches!(cursor_line, cursor_char, instr_start, instr_end) {
                    comp_items = Some(filtered_comp_list(instr_comps));
                } else if cursor_matches!(cursor_line, cursor_char, reg_start, reg_end) {
                    comp_items = Some(filtered_comp_list(reg_comps));
                }

                if let Some(items) = comp_items {
                    return Some(CompletionList {
                        is_incomplete: true,
                        items,
                    });
                }
            }
        }

        // Instruction and one non-register argument
        static QUERY_INSTR_ARG: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(
                tree_sitter_asm::language(),
                "(instruction kind: (word) @instr_name (ident))",
            )
            .unwrap()
        });
        let matches: Vec<tree_sitter::QueryMatch<'_, '_>> = cursor
            .matches(&QUERY_INSTR_ARG, tree.root_node(), curr_doc)
            .collect();
        if let Some(match_) = matches.first() {
            let caps = match_.captures;
            if caps.len() == 1 {
                let instr_start = caps[0].node.range().start_point;
                let instr_end = caps[0].node.range().end_point;
                if cursor_matches!(cursor_line, cursor_char, instr_start, instr_end) {
                    comp_items = Some(filtered_comp_list(instr_comps));
                }

                if let Some(items) = comp_items {
                    return Some(CompletionList {
                        is_incomplete: true,
                        items,
                    });
                }
            }
        }

        // Just an instruction
        static QUERY_INSTR: Lazy<tree_sitter::Query> = Lazy::new(|| {
            tree_sitter::Query::new(
                tree_sitter_asm::language(),
                "(instruction kind: (word) @instr_name)",
            )
            .unwrap()
        });
        let matches: Vec<tree_sitter::QueryMatch<'_, '_>> = cursor
            .matches(&QUERY_INSTR, tree.root_node(), curr_doc)
            .collect();
        if let Some(match_) = matches.first() {
            let caps = match_.captures;
            if caps.len() == 1 {
                let instr_start = caps[0].node.range().start_point;
                let instr_end = caps[0].node.range().end_point;
                if cursor_matches!(cursor_line, cursor_char, instr_start, instr_end) {
                    comp_items = Some(filtered_comp_list(instr_comps));
                }

                if let Some(items) = comp_items {
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
    _params: &DocumentSymbolParams,
) -> Option<Vec<DocumentSymbol>> {
    let tree = parser.parse(curr_doc, None)?;

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

    let mut res: Vec<DocumentSymbol> = vec![];
    let mut cursor = tree.walk();
    loop {
        explore_node(curr_doc, cursor.node(), &mut res);
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    Some(res)
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
