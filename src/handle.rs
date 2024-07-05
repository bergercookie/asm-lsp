use std::path::PathBuf;

use anyhow::{anyhow, Result};
use lsp_server::{Connection, Message, RequestId, Response};
use lsp_textdocument::FullTextDocument;
use lsp_types::{
    CompletionItem, CompletionParams, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams, HoverParams,
    ReferenceParams, SignatureHelpParams,
};
use serde_json::json;
use tree_sitter::{Parser, Tree};

use crate::{
    get_comp_resp, get_document_symbols, get_goto_def_resp, get_hover_resp, get_ref_resp,
    get_sig_help_resp, get_word_from_pos_params, text_doc_change_to_ts_edit, NameToInfoMaps,
    NameToInstructionMap,
};

/// Handles hover requests
///
/// # Errors
///
/// Returns 'Err' if the response fails to send via `connection`
///
/// # Panics
///
/// Panics if JSON encoding of a response fails
pub fn handle_hover_request(
    connection: &Connection,
    id: RequestId,
    params: &HoverParams,
    curr_doc: &Option<FullTextDocument>,
    names_to_info: &NameToInfoMaps,
    include_dirs: &[PathBuf],
) -> Result<()> {
    let empty_resp = Response {
        id: id.clone(),
        result: Some(json!("")),
        error: None,
    };

    let (word, file_word) = if let Some(ref doc) = curr_doc {
        (
            // get the word under the cursor
            get_word_from_pos_params(doc, &params.text_document_position_params, ""),
            // treat the word under the cursor as a filename and grab it as well
            get_word_from_pos_params(doc, &params.text_document_position_params, "."),
        )
    } else {
        return Ok(connection
            .sender
            .send(Message::Response(empty_resp.clone()))?);
    };

    if let Some(hover_resp) = get_hover_resp(
        word,
        file_word,
        &names_to_info.instructions,
        &names_to_info.registers,
        &names_to_info.directives,
        include_dirs,
    ) {
        let result = serde_json::to_value(hover_resp).unwrap();
        let result = Response {
            id,
            result: Some(result),
            error: None,
        };
        return Ok(connection.sender.send(Message::Response(result))?);
    }

    Ok(connection.sender.send(Message::Response(empty_resp))?)
}

/// Handles completion requests
///
/// # Errors
///
/// Returns 'Err' if the response fails to send via `connection`
///
/// # Panics
///
/// Panics if JSON encoding of a response fails
#[allow(clippy::too_many_arguments)]
pub fn handle_completion_request(
    connection: &Connection,
    id: RequestId,
    params: &CompletionParams,
    curr_doc: &Option<FullTextDocument>,
    parser: &mut Parser,
    tree: &mut Option<Tree>,
    instruction_completion_items: &[CompletionItem],
    directive_completion_items: &[CompletionItem],
    register_completion_items: &[CompletionItem],
) -> Result<()> {
    if let Some(ref doc) = curr_doc {
        if let Some(comp_resp) = get_comp_resp(
            doc.get_content(None),
            parser,
            tree,
            params,
            instruction_completion_items,
            directive_completion_items,
            register_completion_items,
        ) {
            let result = serde_json::to_value(comp_resp).unwrap();
            let result = Response {
                id,
                result: Some(result),
                error: None,
            };
            return Ok(connection.sender.send(Message::Response(result))?);
        }
    }

    let empty_resp = Response {
        id,
        result: Some(json!("")),
        error: None,
    };

    Ok(connection.sender.send(Message::Response(empty_resp))?)
}

/// Handles go to definition requests
///
/// # Errors
///
/// Returns 'Err' if the response fails to send via `connection`
///
/// # Panics
///
/// Panics if JSON encoding of a response fails
pub fn handle_goto_def_request(
    connection: &Connection,
    id: RequestId,
    params: &GotoDefinitionParams,
    curr_doc: &Option<FullTextDocument>,
    parser: &mut Parser,
    tree: &mut Option<Tree>,
) -> Result<()> {
    if let Some(ref doc) = curr_doc {
        if let Some(def_resp) = get_goto_def_resp(doc, parser, tree, params) {
            let result = serde_json::to_value(def_resp).unwrap();
            let result = Response {
                id,
                result: Some(result),
                error: None,
            };

            return Ok(connection.sender.send(Message::Response(result))?);
        }
    }

    let empty_resp = Response {
        id,
        result: Some(json!("")),
        error: None,
    };

    Ok(connection.sender.send(Message::Response(empty_resp))?)
}

/// Handles document symbols requests
///
/// # Errors
///
/// Returns 'Err' if the response fails to send via `connection`
///
/// # Panics
///
/// Panics if JSON encoding of a response fails
pub fn handle_document_symbols_request(
    connection: &Connection,
    id: RequestId,
    params: &DocumentSymbolParams,
    curr_doc: &Option<FullTextDocument>,
    parser: &mut Parser,
    tree: &mut Option<Tree>,
) -> Result<()> {
    if let Some(ref doc) = curr_doc {
        if let Some(symbols) = get_document_symbols(doc.get_content(None), parser, tree, params) {
            let resp = DocumentSymbolResponse::Nested(symbols);
            let result = serde_json::to_value(resp).unwrap();
            let result = Response {
                id: id.clone(),
                result: Some(result),
                error: None,
            };
            return Ok(connection.sender.send(Message::Response(result))?);
        }
    }

    let empty_resp = Response {
        id,
        result: Some(json!("")),
        error: None,
    };

    Ok(connection.sender.send(Message::Response(empty_resp))?)
}

/// Handles signature help requests
///
/// # Errors
///
/// Returns 'Err' if the response fails to send via `connection`
///
/// # Panics
///
/// Panics if JSON encoding of a response fails
pub fn handle_signature_help_request(
    connection: &Connection,
    id: RequestId,
    params: &SignatureHelpParams,
    curr_doc: &Option<FullTextDocument>,
    parser: &mut Parser,
    tree: &mut Option<Tree>,
    names_to_instructions: &NameToInstructionMap,
) -> Result<()> {
    if let Some(ref doc) = curr_doc {
        let sig_resp = get_sig_help_resp(
            doc.get_content(None),
            parser,
            params,
            tree,
            names_to_instructions,
        );

        if let Some(sig) = sig_resp {
            let result = serde_json::to_value(sig).unwrap();
            let result = Response {
                id: id.clone(),
                result: Some(result),
                error: None,
            };

            return Ok(connection.sender.send(Message::Response(result))?);
        }
    }

    let empty_resp = Response {
        id,
        result: Some(json!("")),
        error: None,
    };

    Ok(connection.sender.send(Message::Response(empty_resp))?)
}

/// Handles reference requests
///
/// # Errors
///
/// Returns 'Err' if the response fails to send via `connection`
///
/// # Panics
///
/// Panics if JSON encoding of a response fails
pub fn handle_references_request(
    connection: &Connection,
    id: RequestId,
    params: &ReferenceParams,
    curr_doc: &Option<FullTextDocument>,
    parser: &mut Parser,
    tree: &mut Option<Tree>,
) -> Result<()> {
    if let Some(ref doc) = curr_doc {
        let ref_resp = get_ref_resp(doc, parser, tree, params);
        if !ref_resp.is_empty() {
            let result = serde_json::to_value(&ref_resp).unwrap();

            let result = Response {
                id: id.clone(),
                result: Some(result),
                error: None,
            };
            return Ok(connection.sender.send(Message::Response(result.clone()))?);
        }
    }

    let empty_resp = Response {
        id,
        result: Some(json!("")),
        error: None,
    };

    Ok(connection.sender.send(Message::Response(empty_resp))?)
}

/// Handles did open text document notifications
///
/// # Errors
///
/// Returns 'Err' if the response fails to send via `connection`
///
/// # Panics
///
/// Panics if JSON encoding of a response fails
pub fn handle_did_open_text_document_notification(
    params: DidOpenTextDocumentParams,
    curr_doc: &mut Option<FullTextDocument>,
    parser: &mut Parser,
    tree: &mut Option<Tree>,
) {
    *curr_doc = Some(FullTextDocument::new(
        params.text_document.language_id,
        params.text_document.version,
        params.text_document.text.clone(),
    ));
    *tree = parser.parse(params.text_document.text, None);
}

/// Handles did change text document notifications
/// Edits are applied to `curr_doc` and `tree`, but `tree` is not
/// re-parsed
///
/// # Errors
///
/// Returns 'Err' if the response fails to send via `connection`
///
/// # Panics
///
/// Panics if JSON encoding of a response fails
pub fn handle_did_change_text_document_notification(
    params: &DidChangeTextDocumentParams,
    curr_doc: &mut Option<FullTextDocument>,
    tree: &mut Option<Tree>,
) -> Result<()> {
    if let Some(ref mut doc) = curr_doc {
        // Sync our in-memory copy of the current buffer
        doc.update(&params.content_changes, params.text_document.version);
        // Update the TS tree
        if let Some(ref mut curr_tree) = tree {
            for change in &params.content_changes {
                match text_doc_change_to_ts_edit(change, doc) {
                    Ok(edit) => {
                        curr_tree.edit(&edit);
                    }
                    Err(e) => {
                        return Err(anyhow!("Bad edit info, failed to edit tree - Error: {e}"));
                    }
                }
            }
        }
    }

    Ok(())
}
