use crate::types::Column;
use lsp_types::TextDocumentPositionParams;
use std::fs::File;
use std::io::BufRead;

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
        .unwrap_or(0) as usize;

    #[allow(clippy::filter_next)]
    let mut end = line_
        .chars()
        .enumerate()
        .skip(col)
        .filter(|&(_, c)| !is_ident_char(c));

    let end = end.next();
    (start, end.map(|(i, _)| i).unwrap_or(col) as usize)
}

pub fn get_word_from_file_params(pos_params: &TextDocumentPositionParams) -> Result<String, ()> {
    let uri = &pos_params.text_document.uri;
    let line = pos_params.position.line as usize;
    let col = pos_params.position.character as usize;

    let file = File::open(uri.to_file_path()?).expect(&format!("Couldn't open file -> {}", uri));
    let buf_reader = std::io::BufReader::new(file);

    let line_conts = buf_reader.lines().nth(line).unwrap().unwrap();
    let (start, end) = find_word_at_pos(&line_conts, col);
    Ok(String::from(&line_conts[start..end]))
}
