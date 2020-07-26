use asm_lsp::*;

use log::{error, info};
use lsp_types::request::HoverRequest;
use lsp_types::*;

use lsp_server::{Connection, Message, Request, RequestId, Response};
use serde_json::json;

// main -------------------------------------------------------------------------------------------
pub fn main() -> anyhow::Result<()> {
    // initialisation -----------------------------------------------------------------------------
    // Set up logging. Because `stdio_transport` gets a lock on stdout and stdin, we must have our
    // logging only write out to stderr.
    flexi_logger::Logger::with_str("info").start()?;

    // create a map of &Instruction_name -> &Instruction - Use that in user queries
    // The Instruction(s) themselves are stored in a vector and we only keep references to the
    // former map
    info!("Populating instruction set -> x86...");
    let xml_conts_x86 = include_str!("../../opcodes/x86.xml");
    let x86_instructions =
        populate_instructions(&xml_conts_x86)?
            .into_iter()
            .map(|mut instruction| {
                instruction.arch = Some(Arch::X86);
                instruction
            }).collect();

    info!("Populating instruction set -> x86_64...");
    let xml_conts_x86_64 = include_str!("../../opcodes/x86_64.xml");
    let x86_64_instructions =
        populate_instructions(&xml_conts_x86_64)?
            .into_iter()
            .map(|mut instruction| {
                instruction.arch = Some(Arch::X86_64);
                instruction
            }).collect();

    // TODO - Currently in case a name exists both in x86 and x86_64 the latter overrides the
    // former. Modify this to allow to return both
    let mut names_to_instructions = NameToInstructionMap::new();
    populate_name_to_instruction_map(&x86_instructions, &mut names_to_instructions);
    populate_name_to_instruction_map(&x86_64_instructions, &mut names_to_instructions);

    // LSP server initialisation ------------------------------------------------------------------
    info!("Starting lsp server...");

    // Create the transport
    let (connection, io_threads) = Connection::stdio();

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let hover_provider = Some(HoverProviderCapability::Simple(true));
    let capabilities = ServerCapabilities {
        hover_provider,
        ..ServerCapabilities::default()
    };
    let server_capabilities = serde_json::to_value(&capabilities).unwrap();
    let initialization_params = connection.initialize(server_capabilities)?;
    main_loop(&connection, initialization_params, &names_to_instructions)?;
    io_threads.join()?;

    // Shut down gracefully.
    info!("Shutting down lsp server");
    Ok(())
}

fn main_loop(
    connection: &Connection,
    params: serde_json::Value,
    names_to_instructions: &NameToInstructionMap,
) -> anyhow::Result<()> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();
    info!("Starting LSP loop...");
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                match cast::<HoverRequest>(req) {
                    // HoverRequest ---------------------------------------------------------------
                    Ok((id, params)) => {
                        let res = Response {
                            id: id.clone(),
                            result: Some(json!("")),
                            error: None,
                        };

                        // get the word under the cursor
                        let word = get_word_from_file_params(&params.text_document_position_params);

                        // get documentation ------------------------------------------------------
                        // format response
                        let hover_res: Hover;
                        match word {
                            Ok(word) => {
                                match names_to_instructions.get(&*word) {
                                    Some(instruction) => {
                                        // word is a known instruction
                                        hover_res = Hover {
                                            contents: HoverContents::Markup(MarkupContent {
                                                kind: MarkupKind::Markdown,
                                                value: format!("{}", instruction),
                                            }),
                                            range: None,
                                        };
                                        let result = Some(hover_res);
                                        let result = serde_json::to_value(&result).unwrap();
                                        let result = Response {
                                            id: id.clone(),
                                            result: Some(result),
                                            error: None,
                                        };
                                        connection.sender.send(Message::Response(result))?;
                                    }
                                    _ => {
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
                    }
                    Err(req) => {
                        error!("Invalid request format -> {:#?}", req);
                    }
                };
            }
            Message::Response(_resp) => {}
            Message::Notification(_notification) => {}
        }
    }
    Ok(())
}

fn cast<R>(req: Request) -> Result<(RequestId, R::Params), Request>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}
