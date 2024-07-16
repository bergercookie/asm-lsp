use std::collections::HashMap;
use std::path::PathBuf;

use asm_lsp::handle::{
    handle_completion_request, handle_did_change_text_document_notification,
    handle_did_close_text_document_notification, handle_did_open_text_document_notification,
    handle_document_symbols_request, handle_goto_def_request, handle_hover_request,
    handle_references_request, handle_signature_help_request,
};
use asm_lsp::{
    get_compile_cmds, get_completes, get_include_dirs, get_target_config, instr_filter_targets,
    populate_name_to_directive_map, populate_name_to_instruction_map,
    populate_name_to_register_map, Arch, Assembler, Instruction, NameToInfoMaps, TreeStore,
};

use compile_commands::{CompilationDatabase, SourceFile};
use lsp_types::notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument};
use lsp_types::request::{
    Completion, DocumentSymbolRequest, GotoDefinition, HoverRequest, References,
    SignatureHelpRequest,
};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionOptionsCompletionItem,
    HoverProviderCapability, InitializeParams, OneOf, PositionEncodingKind, ServerCapabilities,
    SignatureHelpOptions, TextDocumentSyncCapability, TextDocumentSyncKind,
    WorkDoneProgressOptions,
};

use anyhow::Result;
use log::{error, info};
use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_textdocument::TextDocuments;

// main -------------------------------------------------------------------------------------------
pub fn main() -> Result<()> {
    // initialisation -----------------------------------------------------------------------------
    // Set up logging. Because `stdio_transport` gets a lock on stdout and stdin, we must have our
    // logging only write out to stderr.
    flexi_logger::Logger::try_with_str("info")?.start()?;

    // LSP server initialisation ------------------------------------------------------------------
    info!("Starting asm_lsp...");

    // Create the transport
    let (connection, io_threads) = Connection::stdio();

    // specify UTF-16 encoding for compatibility with lsp-textdocument
    let position_encoding = Some(PositionEncodingKind::UTF16);

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let hover_provider = Some(HoverProviderCapability::Simple(true));

    let completion_provider = Some(CompletionOptions {
        completion_item: Some(CompletionOptionsCompletionItem {
            label_details_support: Some(true),
        }),
        trigger_characters: Some(vec![String::from("%"), String::from(".")]),
        ..Default::default()
    });

    let definition_provider = Some(OneOf::Left(true));

    let text_document_sync = Some(TextDocumentSyncCapability::Kind(
        TextDocumentSyncKind::INCREMENTAL,
    ));

    let signature_help_provider = Some(SignatureHelpOptions {
        trigger_characters: None,
        retrigger_characters: None,
        work_done_progress_options: WorkDoneProgressOptions {
            work_done_progress: Some(false),
        },
    });

    let references_provider = Some(OneOf::Left(true));

    let capabilities = ServerCapabilities {
        position_encoding,
        hover_provider,
        completion_provider,
        signature_help_provider,
        definition_provider,
        text_document_sync,
        document_symbol_provider: Some(OneOf::Left(true)),
        references_provider,
        ..ServerCapabilities::default()
    };
    let server_capabilities = serde_json::to_value(capabilities).unwrap();
    let initialization_params = connection.initialize(server_capabilities)?;

    let mut names_to_info = NameToInfoMaps::default();
    let params: InitializeParams = serde_json::from_value(initialization_params.clone()).unwrap();
    let target_config = get_target_config(&params);
    info!("Server Configuration: {:?}", target_config);

    // create a map of &Instruction_name -> &Instruction - Use that in user queries
    // The Instruction(s) themselves are stored in a vector and we only keep references to the
    // former map
    let x86_instructions = if target_config.instruction_sets.x86 {
        let start = std::time::Instant::now();
        let x86_instrs = include_bytes!("../../docs_store/opcodes/serialized/x86");
        let instrs = bincode::deserialize::<Vec<Instruction>>(x86_instrs)?
            .into_iter()
            .map(|instruction| {
                // filter out assemblers by user config
                instr_filter_targets(&instruction, &target_config)
            })
            .filter(|instruction| !instruction.forms.is_empty())
            .collect();
        info!(
            "x86 instruction set loaded in {}ms",
            start.elapsed().as_millis()
        );
        instrs
    } else {
        Vec::new()
    };

    let x86_64_instructions = if target_config.instruction_sets.x86_64 {
        let start = std::time::Instant::now();
        let x86_64_instrs = include_bytes!("../../docs_store/opcodes/serialized/x86_64");
        let instrs = bincode::deserialize::<Vec<Instruction>>(x86_64_instrs)?
            .into_iter()
            .map(|instruction| {
                // filter out assemblers by user config
                instr_filter_targets(&instruction, &target_config)
            })
            .filter(|instruction| !instruction.forms.is_empty())
            .collect();
        info!(
            "x86-64 instruction set loaded in {}ms",
            start.elapsed().as_millis()
        );
        instrs
    } else {
        Vec::new()
    };

    let z80_instructions = if target_config.instruction_sets.z80 {
        let start = std::time::Instant::now();
        let z80_instrs = include_bytes!("../../docs_store/opcodes/serialized/z80");
        let instrs = bincode::deserialize::<Vec<Instruction>>(z80_instrs)?
            .into_iter()
            .map(|instruction| {
                // filter out assemblers by user config
                instr_filter_targets(&instruction, &target_config)
            })
            .filter(|instruction| !instruction.forms.is_empty())
            .collect();
        info!(
            "z80 instruction set loaded in {}ms",
            start.elapsed().as_millis()
        );
        instrs
    } else {
        Vec::new()
    };

    populate_name_to_instruction_map(
        Arch::X86,
        &x86_instructions,
        &mut names_to_info.instructions,
    );
    populate_name_to_instruction_map(
        Arch::X86_64,
        &x86_64_instructions,
        &mut names_to_info.instructions,
    );
    populate_name_to_instruction_map(
        Arch::Z80,
        &z80_instructions,
        &mut names_to_info.instructions,
    );

    // create a map of &Register_name -> &Register - Use that in user queries
    // The Register(s) themselves are stored in a vector and we only keep references to the
    // former map
    let x86_registers = if target_config.instruction_sets.x86 {
        let start = std::time::Instant::now();
        let regs_x86 = include_bytes!("../../docs_store/registers/serialized/x86");
        let regs = bincode::deserialize(regs_x86)?;
        info!(
            "x86 register set loaded in {}ms",
            start.elapsed().as_millis()
        );
        regs
    } else {
        Vec::new()
    };

    let x86_64_registers = if target_config.instruction_sets.x86_64 {
        let start = std::time::Instant::now();
        let regs_x86_64 = include_bytes!("../../docs_store/registers/serialized/x86_64");
        let regs = bincode::deserialize(regs_x86_64)?;
        info!(
            "x86-64 register set loaded in {}ms",
            start.elapsed().as_millis()
        );
        regs
    } else {
        Vec::new()
    };

    let z80_registers = if target_config.instruction_sets.z80 {
        let start = std::time::Instant::now();
        let regs_z80 = include_bytes!("../../docs_store/registers/serialized/z80");
        let regs = bincode::deserialize(regs_z80)?;
        info!(
            "z80 register set loaded in {}ms",
            start.elapsed().as_millis()
        );
        regs
    } else {
        Vec::new()
    };

    populate_name_to_register_map(Arch::X86, &x86_registers, &mut names_to_info.registers);
    populate_name_to_register_map(
        Arch::X86_64,
        &x86_64_registers,
        &mut names_to_info.registers,
    );
    populate_name_to_register_map(Arch::Z80, &z80_registers, &mut names_to_info.registers);

    let gas_directives = if target_config.assemblers.gas {
        let start = std::time::Instant::now();
        let gas_dirs = include_bytes!("../../docs_store/directives/serialized/gas");
        let dirs = bincode::deserialize(gas_dirs)?;
        info!(
            "Gas directive set loaded in {}ms",
            start.elapsed().as_millis()
        );
        dirs
    } else {
        Vec::new()
    };

    populate_name_to_directive_map(
        Assembler::Gas,
        &gas_directives,
        &mut names_to_info.directives,
    );

    let instr_completion_items = get_completes(
        &names_to_info.instructions,
        Some(CompletionItemKind::OPERATOR),
    );
    let reg_completion_items =
        get_completes(&names_to_info.registers, Some(CompletionItemKind::VARIABLE));
    let directive_completion_items = get_completes(
        &names_to_info.directives,
        Some(CompletionItemKind::OPERATOR),
    );

    let compile_cmds = get_compile_cmds(&params).unwrap_or_default();
    let include_dirs = get_include_dirs(&compile_cmds);

    main_loop(
        &connection,
        initialization_params,
        &names_to_info,
        &instr_completion_items,
        &directive_completion_items,
        &reg_completion_items,
        &compile_cmds,
        &include_dirs,
    )?;
    io_threads.join()?;

    info!("Shutting down asm_lsp");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn main_loop(
    connection: &Connection,
    params: serde_json::Value,
    names_to_info: &NameToInfoMaps,
    instruction_completion_items: &[CompletionItem],
    directive_completion_items: &[CompletionItem],
    register_completion_items: &[CompletionItem],
    _compile_cmds: &CompilationDatabase,
    include_dirs: &HashMap<SourceFile, Vec<PathBuf>>,
) -> Result<()> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();
    let mut text_store = TextDocuments::new();
    let mut tree_store = TreeStore::new();

    info!("Starting asm_lsp loop...");
    for msg in &connection.receiver {
        let start = std::time::Instant::now();
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    info!("Shutting down asm_lsp");
                    return Ok(());
                } else if let Ok((id, params)) = cast_req::<HoverRequest>(req.clone()) {
                    handle_hover_request(
                        connection,
                        id,
                        &params,
                        &text_store,
                        names_to_info,
                        include_dirs,
                    )?;
                    info!(
                        "Hover request serviced in {}ms",
                        start.elapsed().as_millis()
                    );
                } else if let Ok((id, params)) = cast_req::<Completion>(req.clone()) {
                    handle_completion_request(
                        connection,
                        id,
                        &params,
                        &text_store,
                        &mut tree_store,
                        instruction_completion_items,
                        directive_completion_items,
                        register_completion_items,
                    )?;
                    info!(
                        "Completion request serviced in {}ms",
                        start.elapsed().as_millis()
                    );
                } else if let Ok((id, params)) = cast_req::<GotoDefinition>(req.clone()) {
                    handle_goto_def_request(connection, id, &params, &text_store, &mut tree_store)?;
                    info!(
                        "Goto definition request serviced in {}ms",
                        start.elapsed().as_millis()
                    );
                } else if let Ok((id, params)) = cast_req::<DocumentSymbolRequest>(req.clone()) {
                    handle_document_symbols_request(
                        connection,
                        id,
                        &params,
                        &text_store,
                        &mut tree_store,
                    )?;
                    info!(
                        "Document symbols request serviced in {}ms",
                        start.elapsed().as_millis()
                    );
                } else if let Ok((id, params)) = cast_req::<SignatureHelpRequest>(req.clone()) {
                    handle_signature_help_request(
                        connection,
                        id,
                        &params,
                        &text_store,
                        &mut tree_store,
                        &names_to_info.instructions,
                    )?;
                    info!(
                        "Signature help request serviced in {}ms",
                        start.elapsed().as_millis()
                    );
                } else if let Ok((id, params)) = cast_req::<References>(req.clone()) {
                    handle_references_request(
                        connection,
                        id,
                        &params,
                        &text_store,
                        &mut tree_store,
                    )?;
                    info!(
                        "References request serviced in {}ms",
                        start.elapsed().as_millis()
                    );
                } else {
                    error!("Invalid request format -> {:#?}", req);
                }
            }
            Message::Notification(notif) => {
                if let Ok(params) = cast_notif::<DidOpenTextDocument>(notif.clone()) {
                    handle_did_open_text_document_notification(
                        &params,
                        &mut text_store,
                        &mut tree_store,
                    );
                    info!(
                        "Did open text document notification serviced in {}ms",
                        start.elapsed().as_millis()
                    );
                } else if let Ok(params) = cast_notif::<DidChangeTextDocument>(notif.clone()) {
                    handle_did_change_text_document_notification(
                        &params,
                        &mut text_store,
                        &mut tree_store,
                    )?;
                } else if let Ok(params) = cast_notif::<DidCloseTextDocument>(notif.clone()) {
                    handle_did_close_text_document_notification(
                        &params,
                        &mut text_store,
                        &mut tree_store,
                    );
                    info!(
                        "Did close text document notification serviced in {}ms",
                        start.elapsed().as_millis()
                    );
                }
            }
            Message::Response(_resp) => {}
        }
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
