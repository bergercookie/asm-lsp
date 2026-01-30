use anyhow::{Result, anyhow};
use compile_commands::{CompilationDatabase, SourceFile};
use log::{error, info, warn};
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    CompletionParams, Diagnostic, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams,
    HoverParams, PublishDiagnosticsParams, ReferenceParams, SignatureHelpParams, Uri,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
        Notification as _, PublishDiagnostics,
    },
    request::{
        Completion, DocumentDiagnosticRequest, DocumentSymbolRequest, GotoDefinition, HoverRequest,
        References, Request as RequestMessage, SignatureHelpRequest,
    },
};
use tree_sitter::Parser;

use crate::{
    CompletionItems, Config, ConfigOptions, DocumentStore, NameToInstructionMap, RootConfig,
    ServerStore, TreeEntry, UriConversion, apply_compile_cmd, get_comp_resp,
    get_compile_cmd_for_req, get_default_compile_cmd, get_document_symbols, get_goto_def_resp,
    get_hover_resp, get_ref_resp, get_sig_help_resp, get_word_from_pos_params, process_uri,
    send_empty_resp, text_doc_change_to_ts_edit,
};

// A bug in Neovim can cause client->server RPC messages to be corrupted. If this
// happens, log the error and return instead of panicking.
macro_rules! cast_req {
    ($req:expr, $r:ty) => {{
        let Ok(request) = cast_req::<$r>($req) else {
            error!("Failed to cast request of type {}", <$r>::METHOD);
            return Ok(());
        };
        request
    }};
}

macro_rules! cast_notif {
    ($notif:expr, $r:ty) => {{
        let Ok(notification) = cast_notif::<$r>($notif) else {
            error!("Failed to cast notification of type {}", <$r>::METHOD);
            return Ok(());
        };
        notification
    }};
}

/// Handles `Request`s from the lsp client
///
/// # Errors
///
/// Returns errors from any of the handler functions. The majority of error sources
/// are failures to send a response via `connection`
///
/// # Panics
///
/// Panics if JSON encoding of a response fails, a json request fails to cast into
/// its equivalent in memory struct, or the server detects it is in an invalid state
pub fn handle_request(
    req: Request,
    connection: &Connection,
    config: &RootConfig,
    doc_store: &mut DocumentStore,
    store: &ServerStore,
) -> Result<()> {
    let start = std::time::Instant::now();
    match req.method.as_str() {
        HoverRequest::METHOD => {
            let (id, params) = cast_req!(req, HoverRequest);
            handle_hover_request(
                connection,
                id,
                config.get_config(&params.text_document_position_params.text_document.uri),
                &params,
                doc_store,
                store,
            )?;
            info!(
                "{} request serviced in {}ms",
                HoverRequest::METHOD,
                start.elapsed().as_millis()
            );
        }
        Completion::METHOD => {
            let (id, params) = cast_req!(req, Completion);
            handle_completion_request(
                connection,
                id,
                &params,
                config.get_config(&params.text_document_position.text_document.uri),
                doc_store,
                &store.completion_items,
            )?;
            info!(
                "{} request serviced in {}ms",
                Completion::METHOD,
                start.elapsed().as_millis()
            );
        }
        GotoDefinition::METHOD => {
            let (id, params) = cast_req!(req, GotoDefinition);
            handle_goto_def_request(connection, id, &params, doc_store)?;
            info!(
                "{} request serviced in {}ms",
                GotoDefinition::METHOD,
                start.elapsed().as_millis()
            );
        }
        DocumentSymbolRequest::METHOD => {
            let (id, params) = cast_req!(req, DocumentSymbolRequest);
            handle_document_symbols_request(connection, id, &params, doc_store)?;
            info!(
                "{} request serviced in {}ms",
                DocumentSymbolRequest::METHOD,
                start.elapsed().as_millis()
            );
        }
        SignatureHelpRequest::METHOD => {
            let (id, params) = cast_req!(req, SignatureHelpRequest);
            handle_signature_help_request(
                connection,
                id,
                &params,
                config.get_config(&params.text_document_position_params.text_document.uri),
                doc_store,
                &store.names_to_info.instructions,
            )?;
            info!(
                "{} request serviced in {}ms",
                SignatureHelpRequest::METHOD,
                start.elapsed().as_millis()
            );
        }
        References::METHOD => {
            let (id, params) = cast_req!(req, References);
            handle_references_request(connection, id, &params, doc_store)?;
            info!(
                "{} request serviced in {}ms",
                References::METHOD,
                start.elapsed().as_millis()
            );
        }
        DocumentDiagnosticRequest::METHOD => {
            let (_id, params) = cast_req!(req, DocumentDiagnosticRequest);
            let project_config = config.get_config(&params.text_document.uri);
            // Ok to unwrap, this should never be `None`
            if project_config.opts.as_ref().unwrap().diagnostics.unwrap() {
                let compile_cmds = get_compile_cmd_for_req(
                    config,
                    &params.text_document.uri,
                    &store.compile_commands,
                );
                info!(
                    "Selected compile command(s) for request: {:?}",
                    compile_cmds
                );
                handle_diagnostics(
                    connection,
                    &params.text_document.uri,
                    project_config,
                    &compile_cmds,
                )?;
                info!(
                    "{} request serviced in {}ms",
                    DocumentDiagnosticRequest::METHOD,
                    start.elapsed().as_millis()
                );
            }
        }
        method => warn!("Invalid request format: {method:?}"),
    }

    Ok(())
}

/// Handles `Notification`s from the lsp client
///
/// # Errors
///
/// Returns errors from any of the handler functions.
///
/// # Panics
///
/// Panics if JSON encoding of a response fails, a json request fails to cast into
/// its equivalent in memory struct, or the server detects it is in an invalid state
pub fn handle_notification(
    notif: Notification,
    connection: &Connection,
    doc_store: &mut DocumentStore,
    config: &RootConfig,
    store: &ServerStore,
) -> Result<()> {
    let start = std::time::Instant::now();
    match notif.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params = cast_notif!(notif, DidOpenTextDocument);
            handle_did_open_text_document_notification(&params, doc_store);
            info!(
                "{} notification serviced in {}ms",
                DidOpenTextDocument::METHOD,
                start.elapsed().as_millis()
            );
        }
        DidChangeTextDocument::METHOD => {
            let params = cast_notif!(notif, DidChangeTextDocument);
            handle_did_change_text_document_notification(&params, doc_store)?;
            info!(
                "{} notification serviced in {}ms",
                DidChangeTextDocument::METHOD,
                start.elapsed().as_millis()
            );
        }
        DidCloseTextDocument::METHOD => {
            let params = cast_notif!(notif, DidCloseTextDocument);
            handle_did_close_text_document_notification(&params, doc_store);
            info!(
                "{} notification serviced in {}ms",
                DidCloseTextDocument::METHOD,
                start.elapsed().as_millis()
            );
        }
        DidSaveTextDocument::METHOD => {
            let params = cast_notif!(notif, DidSaveTextDocument);
            let project_config = config.get_config(&params.text_document.uri);
            // Ok to unwrap, this should never be `None`
            if project_config.opts.as_ref().unwrap().diagnostics.unwrap() {
                let compile_cmds = get_compile_cmd_for_req(
                    config,
                    &params.text_document.uri,
                    &store.compile_commands,
                );
                info!(
                    "Selected compile command(s) for request: {:?}",
                    compile_cmds
                );
                handle_diagnostics(
                    connection,
                    &params.text_document.uri,
                    project_config,
                    &compile_cmds,
                )?;
                info!(
                    "Published diagnostics on save in {}ms",
                    start.elapsed().as_millis()
                );
            }
        }
        method => warn!("Invalid notification format: {method:?}"),
    }
    Ok(())
}

fn cast_req<R>(req: Request) -> Result<(RequestId, R::Params)>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    match req.extract(R::METHOD) {
        Ok(value) => Ok(value),
        // Fixme please
        Err(e) => Err(anyhow::anyhow!("Error: {e}")),
    }
}

fn cast_notif<R>(notif: Notification) -> Result<R::Params>
where
    R: lsp_types::notification::Notification,
    R::Params: serde::de::DeserializeOwned,
{
    match notif.extract(R::METHOD) {
        Ok(value) => Ok(value),
        // Fixme please
        Err(e) => Err(anyhow::anyhow!("Error: {e}")),
    }
}

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
    doc_store: &mut DocumentStore,
    store: &ServerStore,
) -> Result<()> {
    let (word, cursor_offset) = if let Some(doc) = doc_store
        .text_store
        .get_document(&params.text_document_position_params.text_document.uri)
    {
        get_word_from_pos_params(doc, &params.text_document_position_params)
    } else {
        return send_empty_resp(connection, id);
    };

    // needed to appease the borrow checker, since `word` is a reference to owned
    // data inside `doc_store.text_store`, which we're passing as mutable
    let word = word.to_string();
    if let Some(hover_resp) = get_hover_resp(params, config, &word, cursor_offset, doc_store, store)
    {
        let result = serde_json::to_value(hover_resp).unwrap();
        let result = Response {
            id,
            result: Some(result),
            error: None,
        };
        return Ok(connection.sender.send(Message::Response(result))?);
    }

    send_empty_resp(connection, id)
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
    doc_store: &mut DocumentStore,
    completion_items: &CompletionItems,
) -> Result<()> {
    let uri = &params.text_document_position.text_document.uri;
    if let Some(doc) = doc_store.text_store.get_document(uri)
        && let Some(ref mut tree_entry) = doc_store.tree_store.get_mut(uri)
        && let Some(comp_resp) = get_comp_resp(
            doc.get_content(None),
            tree_entry,
            params,
            config,
            completion_items,
        )
    {
        let result = serde_json::to_value(comp_resp).unwrap();
        let result = Response {
            id,
            result: Some(result),
            error: None,
        };
        return Ok(connection.sender.send(Message::Response(result))?);
    }

    send_empty_resp(connection, id)
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
    doc_store: &mut DocumentStore,
) -> Result<()> {
    let uri = &params.text_document_position_params.text_document.uri;
    if let Some(doc) = doc_store.text_store.get_document(uri)
        && let Some(tree_entry) = doc_store.tree_store.get_mut(uri)
        && let Some(def_resp) = get_goto_def_resp(doc, tree_entry, params)
    {
        let result = serde_json::to_value(def_resp).unwrap();
        let result = Response {
            id,
            result: Some(result),
            error: None,
        };

        return Ok(connection.sender.send(Message::Response(result))?);
    }

    send_empty_resp(connection, id)
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
    doc_store: &mut DocumentStore,
) -> Result<()> {
    let uri = &params.text_document.uri;
    if let Some(doc) = doc_store.text_store.get_document(uri)
        && let Some(tree_entry) = doc_store.tree_store.get_mut(uri)
        && let Some(symbols) = get_document_symbols(doc.get_content(None), tree_entry, params)
    {
        let resp = DocumentSymbolResponse::Nested(symbols);
        let result = serde_json::to_value(resp).unwrap();
        let result = Response {
            id,
            result: Some(result),
            error: None,
        };
        return Ok(connection.sender.send(Message::Response(result))?);
    }

    send_empty_resp(connection, id)
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
    doc_store: &mut DocumentStore,
    names_to_instructions: &NameToInstructionMap,
) -> Result<()> {
    let uri = &params.text_document_position_params.text_document.uri;
    if let Some(doc) = doc_store.text_store.get_document(uri)
        && let Some(tree_entry) = doc_store.tree_store.get_mut(uri)
        && let Some(sig_resp) = get_sig_help_resp(
            doc.get_content(None),
            params,
            config,
            tree_entry,
            names_to_instructions,
        )
    {
        let result = serde_json::to_value(sig_resp).unwrap();
        let result = Response {
            id,
            result: Some(result),
            error: None,
        };
        return Ok(connection.sender.send(Message::Response(result))?);
    }

    send_empty_resp(connection, id)
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
    doc_store: &mut DocumentStore,
) -> Result<()> {
    let uri = &params.text_document_position.text_document.uri;
    if let Some(doc) = doc_store.text_store.get_document(uri)
        && let Some(tree_entry) = doc_store.tree_store.get_mut(uri)
    {
        let ref_resp = get_ref_resp(params, doc, tree_entry);
        let result = serde_json::to_value(&ref_resp).unwrap();

        let result = Response {
            id,
            result: Some(result),
            error: None,
        };
        return Ok(connection.sender.send(Message::Response(result))?);
    }

    send_empty_resp(connection, id)
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
    let req_source_path = match process_uri(uri) {
        UriConversion::Canonicalized(p) => p,
        UriConversion::Unchecked(p) => {
            error!(
                "Failed to canonicalize request path {}, using {}",
                uri.path().as_str(),
                p.display()
            );
            p
        }
    };

    let source_entries = compile_cmds.iter().filter(|entry| match entry.file {
        SourceFile::File(ref file) => {
            file.canonicalize().is_ok_and(|source_path| {
                // HACK: See comment inside `process_uri`
                let cleaned_path = if cfg!(windows) {
                    #[allow(clippy::option_if_let_else)]
                    if let Some(tmp) = source_path.to_str().unwrap().strip_prefix("\\\\?\\") {
                        warn!("Stripping Windows canonicalization prefix \"\\\\?\\\" from path");
                        tmp.into()
                    } else {
                        source_path
                    }
                } else {
                    source_path
                };
                cleaned_path.eq(&req_source_path)
            })
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
    doc_store: &mut DocumentStore,
) {
    let raw_params = serde_json::to_value(params).unwrap();
    doc_store
        .text_store
        .listen(DidOpenTextDocument::METHOD, &raw_params);

    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_asm::language()).unwrap();
    doc_store.tree_store.insert(
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
    doc_store: &mut DocumentStore,
) -> Result<()> {
    let raw_params = serde_json::to_value(params).unwrap();
    doc_store
        .text_store
        .listen(DidChangeTextDocument::METHOD, &raw_params);

    let uri = &params.text_document.uri;
    if let Some(ref mut doc) = doc_store.text_store.get_document(uri)
        && let Some(tree_entry) = doc_store.tree_store.get_mut(uri)
        && let Some(ref mut curr_tree) = tree_entry.tree
    {
        for change in &params.content_changes {
            match text_doc_change_to_ts_edit(change, doc) {
                Ok(edit) => curr_tree.edit(&edit),
                Err(e) => {
                    return Err(anyhow!("Bad edit info, failed to edit tree - Error: {e}"));
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
    doc_store: &mut DocumentStore,
) {
    let raw_params = serde_json::to_value(params).unwrap();
    doc_store
        .text_store
        .listen(DidCloseTextDocument::METHOD, &raw_params);
    doc_store.tree_store.remove(&params.text_document.uri);
}
