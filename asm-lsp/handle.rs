use std::{collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Result};
use compile_commands::{CompilationDatabase, SourceFile};
use log::info;
use lsp_server::{Connection, Message, RequestId, Response};
use lsp_textdocument::TextDocuments;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
        PublishDiagnostics,
    },
    CompletionItem, CompletionParams, Diagnostic, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentSymbolParams,
    DocumentSymbolResponse, GotoDefinitionParams, HoverParams, PublishDiagnosticsParams,
    ReferenceParams, SignatureHelpParams, Uri,
};
use tree_sitter::Parser;

use crate::{
    apply_compile_cmd, get_comp_resp, get_default_compile_cmd, get_document_symbols,
    get_goto_def_resp, get_hover_resp, get_ref_resp, get_sig_help_resp, get_word_from_pos_params,
    send_empty_resp, text_doc_change_to_ts_edit, Arch, Assembler, Config, ConfigOptions,
    NameToInfoMaps, NameToInstructionMap, TreeEntry, TreeStore,
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
    config: &Config,
    params: &HoverParams,
    text_store: &TextDocuments,
    tree_store: &mut TreeStore,
    names_to_info: &NameToInfoMaps,
    include_dirs: &HashMap<SourceFile, Vec<PathBuf>>,
) -> Result<()> {
    let (word, cursor_offset) = if let Some(doc) =
        text_store.get_document(&params.text_document_position_params.text_document.uri)
    {
        get_word_from_pos_params(doc, &params.text_document_position_params)
    } else {
        return send_empty_resp(connection, id, config);
    };

    if let Some(hover_resp) = get_hover_resp(
        params,
        config,
        word,
        cursor_offset,
        text_store,
        tree_store,
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

    send_empty_resp(connection, id, config)
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
pub fn handle_completion_request(
    connection: &Connection,
    id: RequestId,
    params: &CompletionParams,
    config: &Config,
    text_store: &TextDocuments,
    tree_store: &mut TreeStore,
    instruction_completion_items: &[(Arch, CompletionItem)],
    directive_completion_items: &[(Assembler, CompletionItem)],
    register_completion_items: &[(Arch, CompletionItem)],
) -> Result<()> {
    let uri = &params.text_document_position.text_document.uri;
    if let Some(doc) = text_store.get_document(uri) {
        if let Some(ref mut tree_entry) = tree_store.get_mut(uri) {
            if let Some(comp_resp) = get_comp_resp(
                doc.get_content(None),
                tree_entry,
                params,
                config,
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
    }

    send_empty_resp(connection, id, config)
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
    config: &Config,
    text_store: &TextDocuments,
    tree_store: &mut TreeStore,
) -> Result<()> {
    let uri = &params.text_document_position_params.text_document.uri;
    if let Some(doc) = text_store.get_document(uri) {
        if let Some(tree_entry) = tree_store.get_mut(uri) {
            if let Some(def_resp) = get_goto_def_resp(doc, tree_entry, params) {
                let result = serde_json::to_value(def_resp).unwrap();
                let result = Response {
                    id,
                    result: Some(result),
                    error: None,
                };

                return Ok(connection.sender.send(Message::Response(result))?);
            }
        }
    }

    send_empty_resp(connection, id, config)
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
    config: &Config,
    text_store: &TextDocuments,
    tree_store: &mut TreeStore,
) -> Result<()> {
    let uri = &params.text_document.uri;
    if let Some(doc) = text_store.get_document(uri) {
        if let Some(tree_entry) = tree_store.get_mut(uri) {
            if let Some(symbols) = get_document_symbols(doc.get_content(None), tree_entry, params) {
                let resp = DocumentSymbolResponse::Nested(symbols);
                let result = serde_json::to_value(resp).unwrap();
                let result = Response {
                    id,
                    result: Some(result),
                    error: None,
                };
                return Ok(connection.sender.send(Message::Response(result))?);
            }
        }
    }

    send_empty_resp(connection, id, config)
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
    config: &Config,
    text_store: &TextDocuments,
    tree_store: &mut TreeStore,
    names_to_instructions: &NameToInstructionMap,
) -> Result<()> {
    let uri = &params.text_document_position_params.text_document.uri;
    if let Some(doc) = text_store.get_document(uri) {
        if let Some(tree_entry) = tree_store.get_mut(uri) {
            let sig_resp = get_sig_help_resp(
                doc.get_content(None),
                params,
                config,
                tree_entry,
                names_to_instructions,
            );

            if let Some(sig) = sig_resp {
                let result = serde_json::to_value(sig).unwrap();
                let result = Response {
                    id,
                    result: Some(result),
                    error: None,
                };

                return Ok(connection.sender.send(Message::Response(result))?);
            }
        }
    }

    send_empty_resp(connection, id, config)
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
    config: &Config,
    text_store: &TextDocuments,
    tree_store: &mut TreeStore,
) -> Result<()> {
    let uri = &params.text_document_position.text_document.uri;
    if let Some(doc) = text_store.get_document(uri) {
        if let Some(tree_entry) = tree_store.get_mut(uri) {
            let ref_resp = get_ref_resp(params, doc, tree_entry);
            if !ref_resp.is_empty() {
                let result = serde_json::to_value(&ref_resp).unwrap();

                let result = Response {
                    id,
                    result: Some(result),
                    error: None,
                };
                return Ok(connection.sender.send(Message::Response(result))?);
            }
        }
    }

    send_empty_resp(connection, id, config)
}

/// Produces diagnostics and sends a `PublishDiagnostics` notification to the client
/// Diagnostics are only produced for the file specified by `uri`
///
/// # Errors
///
/// Returns 'Err' if the response fails to send via `connection`
///
/// # Panics
///
/// Panics if JSON encoding of the notification fails
pub fn handle_diagnostics(
    connection: &Connection,
    uri: &Uri,
    cfg: &Config,
    compile_cmds: &CompilationDatabase,
) -> Result<()> {
    let req_source_path = PathBuf::from(uri.path().as_str());

    let source_entries = compile_cmds.iter().filter(|entry| match entry.file {
        SourceFile::File(ref file) => {
            if file.is_absolute() {
                file.eq(&req_source_path)
            } else if let Ok(source_path) = file.canonicalize() {
                source_path.eq(&req_source_path)
            } else {
                false
            }
        }
        SourceFile::All => true,
    });

    let mut has_entries = false;
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    for entry in source_entries {
        has_entries = true;
        apply_compile_cmd(cfg, &mut diagnostics, uri, entry);
    }

    // If no user-provided entries corresponded to the file, just try out
    // invoking the user-provided compiler (if they gave one), or alternatively
    // gcc (and clang if that fails) with the source file path as the only argument
    if !has_entries
        && matches!(
            cfg.opts,
            // NOTE: We ensure this field is always `Some` at load time
            Some(ConfigOptions {
                // NOTE: We ensure this field is always `Some` at load time
                default_diagnostics: Some(true),
                ..
            })
        )
    {
        info!(
            "No applicable user-provided commands for {}. Applying default compile command",
            uri.path().as_str()
        );
        apply_compile_cmd(
            cfg,
            &mut diagnostics,
            uri,
            &get_default_compile_cmd(uri, cfg),
        );
    }

    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version: None,
    };
    let result = serde_json::to_value(params).unwrap();

    let notif = lsp_server::Notification {
        method: PublishDiagnostics::METHOD.to_string(),
        params: result,
    };
    Ok(connection.sender.send(Message::Notification(notif))?)
}

/// Handles did open text document notifications
///
/// # Errors
///
/// Returns 'Err' if the response fails to send via `connection`
///
/// # Panics
///
/// Panics if JSON encoding of a response fails, or if the parser
/// fails to set the language
pub fn handle_did_open_text_document_notification(
    params: &DidOpenTextDocumentParams,
    text_store: &mut TextDocuments,
    tree_store: &mut TreeStore,
) {
    let raw_params = serde_json::to_value(params).unwrap();
    text_store.listen(DidOpenTextDocument::METHOD, &raw_params);

    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_asm::language()).unwrap();
    tree_store.insert(
        params.text_document.uri.clone(),
        TreeEntry {
            tree: parser.parse(&params.text_document.text, None),
            parser,
        },
    );
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
    text_store: &mut TextDocuments,
    tree_store: &mut TreeStore,
) -> Result<()> {
    let raw_params = serde_json::to_value(params).unwrap();
    text_store.listen(DidChangeTextDocument::METHOD, &raw_params);

    let uri = &params.text_document.uri;
    if let Some(ref mut doc) = text_store.get_document(uri) {
        if let Some(tree_entry) = tree_store.get_mut(uri) {
            if let Some(ref mut curr_tree) = tree_entry.tree {
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
    }

    Ok(())
}

/// Handles did close text document notifications
///
/// # Panics
///
/// Panics if JSON encoding of `params` fails
pub fn handle_did_close_text_document_notification(
    params: &DidCloseTextDocumentParams,
    text_store: &mut TextDocuments,
    tree_store: &mut TreeStore,
) {
    let raw_params = serde_json::to_value(params).unwrap();
    text_store.listen(DidCloseTextDocument::METHOD, &raw_params);
    tree_store.remove(&params.text_document.uri);
}
