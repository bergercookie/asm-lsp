use asm_lsp::*;

use log::{error, info};
use lsp_types::notification::{DidChangeTextDocument, DidOpenTextDocument};
use lsp_types::request::{Completion, DocumentSymbolRequest, HoverRequest};
use lsp_types::*;

use crate::lsp::{get_document_symbols, get_target_config, instr_filter_targets};

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use serde_json::json;

// main -------------------------------------------------------------------------------------------
pub fn main() -> anyhow::Result<()> {
    // initialisation -----------------------------------------------------------------------------
    // Set up logging. Because `stdio_transport` gets a lock on stdout and stdin, we must have our
    // logging only write out to stderr.
    flexi_logger::Logger::try_with_str("info")?.start()?;

    // LSP server initialisation ------------------------------------------------------------------
    info!("Starting LSP server...");

    // Create the transport
    let (connection, io_threads) = Connection::stdio();

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let hover_provider = Some(HoverProviderCapability::Simple(true));

    let completion_provider = Some(CompletionOptions {
        completion_item: Some(CompletionOptionsCompletionItem {
            label_details_support: Some(true),
        }),
        trigger_characters: Some(vec![String::from("%")]),
        ..Default::default()
    });

    let text_document_sync = Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL));

    let capabilities = ServerCapabilities {
        hover_provider,
        completion_provider,
        text_document_sync,
        document_symbol_provider: Some(OneOf::Left(true)),
        ..ServerCapabilities::default()
    };
    let server_capabilities = serde_json::to_value(capabilities).unwrap();
    let initialization_params = connection.initialize(server_capabilities)?;

    let params: InitializeParams = serde_json::from_value(initialization_params.clone()).unwrap();
    let target_config = get_target_config(&params);

    let x86_registers = if target_config.instruction_sets.x86 {
        info!("Populating register set -> x86...");
        let xml_conts_regs_x86 = include_str!("../../registers/x86.xml");
        populate_registers(xml_conts_regs_x86)?
    } else {
        Vec::new()
    };

    let x86_64_registers = if target_config.instruction_sets.x86_64 {
        info!("Populating register set -> x86_64...");
        let xml_conts_regs_x86_64 = include_str!("../../registers/x86_64.xml");
        populate_registers(xml_conts_regs_x86_64)?
    } else {
        Vec::new()
    };

    let mut names_to_registers = NameToRegisterMap::new();
    populate_name_to_register_map(Arch::X86, &x86_registers, &mut names_to_registers);
    populate_name_to_register_map(Arch::X86_64, &x86_64_registers, &mut names_to_registers);

    // create a map of &Instruction_name -> &Instruction - Use that in user queries
    // The Instruction(s) themselves are stored in a vector and we only keep references to the
    // former map
    let x86_instructions = if target_config.instruction_sets.x86 {
        info!("Populating instruction set -> x86...");
        let xml_conts_x86 = include_str!("../../opcodes/x86.xml");
        populate_instructions(xml_conts_x86)?
            .into_iter()
            .map(|mut instruction| {
                instruction.arch = Some(Arch::X86);
                instruction
            })
            .map(|instruction| {
                // filter out assemblers by user config
                instr_filter_targets(&instruction, &target_config)
            })
            .filter(|instruction| !instruction.forms.is_empty())
            .collect()
    } else {
        Vec::new()
    };

    let x86_64_instructions = if target_config.instruction_sets.x86_64 {
        info!("Populating instruction set -> x86_64...");
        let xml_conts_x86_64 = include_str!("../../opcodes/x86_64.xml");
        populate_instructions(xml_conts_x86_64)?
            .into_iter()
            .map(|mut instruction| {
                instruction.arch = Some(Arch::X86_64);
                instruction
            })
            .map(|instruction| {
                // filter out assemblers by user config
                instr_filter_targets(&instruction, &target_config)
            })
            .filter(|instruction| !instruction.forms.is_empty())
            .collect()
    } else {
        Vec::new()
    };

    let mut names_to_instructions = NameToInstructionMap::new();
    populate_name_to_instruction_map(Arch::X86, &x86_instructions, &mut names_to_instructions);
    populate_name_to_instruction_map(
        Arch::X86_64,
        &x86_64_instructions,
        &mut names_to_instructions,
    );

    // create a map of &Register_name -> &Register - Use that in user queries
    // The Register(s) themselves are stored in a vector and we only keep references to the
    // former map
    let x86_registers = if target_config.instruction_sets.x86 {
        info!("Populating register set -> x86...");
        let xml_conts_regs_x86 = include_str!("../../registers/x86.xml");
        populate_registers(xml_conts_regs_x86)?
            .into_iter()
            .map(|mut reg| {
                reg.arch = Some(Arch::X86);
                reg
            })
            .collect()
    } else {
        Vec::new()
    };

    let x86_64_registers = if target_config.instruction_sets.x86_64 {
        info!("Populating register set -> x86_64...");
        let xml_conts_regs_x86_64 = include_str!("../../registers/x86_64.xml");
        populate_registers(xml_conts_regs_x86_64)?
            .into_iter()
            .map(|mut reg| {
                reg.arch = Some(Arch::X86_64);
                reg
            })
            .collect()
    } else {
        Vec::new()
    };

    let mut names_to_registers = NameToRegisterMap::new();
    populate_name_to_register_map(Arch::X86, &x86_registers, &mut names_to_registers);
    populate_name_to_register_map(Arch::X86_64, &x86_64_registers, &mut names_to_registers);

    let instr_comps = get_completes(&names_to_instructions, Some(CompletionItemKind::OPERATOR));
    let reg_comps = get_completes(&names_to_registers, Some(CompletionItemKind::VARIABLE));

    main_loop(
        &connection,
        initialization_params,
        &names_to_instructions,
        &names_to_registers,
        &instr_comps,
        &reg_comps,
    )?;
    io_threads.join()?;

    // Shut down gracefully.
    info!("Shutting down LSP server");
    Ok(())
}

fn main_loop(
    connection: &Connection,
    params: serde_json::Value,
    names_to_instructions: &NameToInstructionMap,
    names_to_registers: &NameToRegisterMap,
    instruction_completes: &[CompletionItem],
    register_completes: &[CompletionItem],
) -> anyhow::Result<()> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();

    let mut doc_open_params;
    let mut doc_change_params;
    let curr_doc = String::new();
    let mut curr_doc = &curr_doc;
    let mut parser = tree_sitter::Parser::new();
    parser.set_logger(Some(Box::new(tree_sitter_logger)));
    parser.set_language(tree_sitter_asm::language())?;

    info!("Starting LSP loop...");
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                } else if let Ok((id, params)) = cast_req::<HoverRequest>(req.clone()) {
                    // HoverRequest ---------------------------------------------------------------
                    let res = Response {
                        id: id.clone(),
                        result: Some(json!("")),
                        error: None,
                    };

                    // get the word under the cursor
                    let word = get_word_from_file_params(&params.text_document_position_params);

                    // get documentation ------------------------------------------------------
                    // format response
                    match word {
                        Ok(word) => {
                            let hover_res = get_hover_resp(&word, names_to_instructions);
                            // If no instructions matched, check the registers
                            let hover_res = if hover_res.is_none() {
                                get_hover_resp(&word, names_to_registers)
                            } else {
                                hover_res
                            };
                            match hover_res {
                                Some(_) => {
                                    let result = serde_json::to_value(&hover_res).unwrap();
                                    let result = Response {
                                        id: id.clone(),
                                        result: Some(result),
                                        error: None,
                                    };
                                    connection.sender.send(Message::Response(result))?;
                                }
                                None => {
                                    // don't know of this word
                                    connection.sender.send(Message::Response(res.clone()))?;
                                }
                            }
                        }
                        Err(_) => {
                            // given word is not valid
                            connection.sender.send(Message::Response(res))?;
                        }
                    }
                } else if let Ok((id, params)) = cast_req::<Completion>(req.clone()) {
                    // CompletionRequest ---------------------------------------------------------------
                    // get suggestions ------------------------------------------------------
                    let comp_res = get_comp_resp(
                        curr_doc,
                        &mut parser,
                        &params,
                        instruction_completes,
                        register_completes,
                    );
                    match comp_res {
                        Some(_) => {
                            let result = serde_json::to_value(&comp_res).unwrap();
                            let result = Response {
                                id: id.clone(),
                                result: Some(result),
                                error: None,
                            };
                            connection.sender.send(Message::Response(result))?;
                        }
                        None => {
                            // don't know what to suggest
                            let res = Response {
                                id: id.clone(),
                                result: Some(json!("")),
                                error: None,
                            };
                            connection.sender.send(Message::Response(res.clone()))?;
                        }
                    }
                } else if let Ok((id, params)) = cast_req::<DocumentSymbolRequest>(req.clone()) {
                    let symbols = get_document_symbols(curr_doc, &mut parser, &params);
                    match symbols {
                        Some(symbols) => {
                            let resp = DocumentSymbolResponse::Nested(symbols);
                            let result = serde_json::to_value(&resp).unwrap();
                            let result = Response {
                                id: id.clone(),
                                result: Some(result),
                                error: None,
                            };
                            connection.sender.send(Message::Response(result))?;
                        }
                        None => {
                            // don't know what to reply
                            let res = Response {
                                id: id.clone(),
                                result: Some(json!("")),
                                error: None,
                            };
                            connection.sender.send(Message::Response(res.clone()))?;
                        }
                    }
                } else {
                    error!("Invalid request fromat -> {:#?}", req);
                }
            }
            Message::Notification(notif) => {
                if let Ok(params) = cast_notif::<DidOpenTextDocument>(notif.clone()) {
                    // move the notification's params into this variable so we can avoid cloning the document contents
                    doc_open_params = params;
                    curr_doc = &doc_open_params.text_document.text;
                } else if let Ok(params) = cast_notif::<DidChangeTextDocument>(notif.clone()) {
                    doc_change_params = params;
                    curr_doc = &doc_change_params.content_changes.last().unwrap().text;
                }
            }
            Message::Response(_resp) => {}
        }
    }
    Ok(())
}

fn cast_req<R>(req: Request) -> anyhow::Result<(RequestId, R::Params)>
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

fn cast_notif<R>(notif: Notification) -> anyhow::Result<R::Params>
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
