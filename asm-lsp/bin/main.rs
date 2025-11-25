use std::thread::sleep;
use std::time::Duration;

use asm_lsp::config_builder::{GenerateArgs, GenerateOpts, gen_config};
use asm_lsp::{Arch, Assembler, run_info};

use asm_lsp::handle::{handle_notification, handle_request};
use asm_lsp::{
    DocumentStore, RootConfig, ServerStore, get_compile_cmds_from_file, get_completes,
    get_include_dirs, get_root_config, send_notification,
};

use clap::{Command, FromArgMatches as _, Subcommand};
use lsp_types::{
    CompletionItemKind, CompletionOptions, CompletionOptionsCompletionItem, DiagnosticOptions,
    DiagnosticServerCapabilities, HoverProviderCapability, InitializeParams, MessageType, OneOf,
    PositionEncodingKind, ServerCapabilities, SignatureHelpOptions, TextDocumentSyncCapability,
    TextDocumentSyncKind, WorkDoneProgressOptions,
};

use anyhow::Result;
use log::{info, warn};
use lsp_server::{Connection, Message};

#[derive(Subcommand)]
enum Commands {
    GenConfig(GenerateArgs),
    /// Print information about asm-lsp
    Info,
    /// Print the version number only
    Version,
    #[clap(hide(true))]
    Run,
}

/// Entry point of the lsp. Runs a subcommand is specified, otherwise starts the
/// lsp server
///
/// # Errors
///
/// Returns `Err` on failure to parse arguments, if a sub command fails, or if the
/// server returns an error
pub fn main() -> Result<()> {
    let cli = Command::new("asm-lsp").subcommand_required(false);
    let cli = Commands::augment_subcommands(cli);

    let command = match Commands::from_arg_matches(&cli.get_matches()) {
        Ok(cmd) => cmd,
        Err(e) => {
            // If no subcommand is provided, run the server as normal
            if e.kind() == clap::error::ErrorKind::MissingSubcommand {
                Commands::Run
            } else {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    };
    match command {
        Commands::GenConfig(args) => {
            let opts: GenerateOpts = match args.try_into() {
                Ok(opts) => opts,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            };

            if let Err(e) = gen_config(&opts) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Commands::Version => println!("{}", env!("CARGO_PKG_VERSION")),
        Commands::Info => run_info(),
        Commands::Run => run_lsp()?,
    }

    Ok(())
}

/// Entry point of the lsp server. Connects to the client, loads documentation resources,
/// and then enters the main loop
///
/// # Errors
///
/// Returns `Err` if the server fails to connect to the lsp client
///
/// # Panics
///
/// Panics if JSON serialization of the server capabilities fails
pub fn run_lsp() -> Result<()> {
    // initialisation
    // Set up logging. Because `stdio_transport` gets a lock on stdout and stdin, we must have our
    // logging only write out to stderr.
    flexi_logger::Logger::try_with_str("info")?.start()?;

    // LSP server initialisation
    info!("Starting asm-lsp-{}", env!("CARGO_PKG_VERSION"));

    // Create the transport
    let (connection, _io_threads) = Connection::stdio();

    // specify UTF-16 encoding for compatibility with lsp-textdocument
    let position_encoding = Some(PositionEncodingKind::UTF16);

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let hover_provider = Some(HoverProviderCapability::Simple(true));

    let completion_provider = Some(CompletionOptions {
        completion_item: Some(CompletionOptionsCompletionItem {
            label_details_support: Some(true),
        }),
        trigger_characters: Some(vec![
            String::from("%"),
            String::from("."),
            String::from("$"),
        ]),
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

    let diagnostic_provider = Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
        identifier: Some(String::from("asm-lsp")),
        inter_file_dependencies: true,
        workspace_diagnostics: false,
        work_done_progress_options: WorkDoneProgressOptions {
            work_done_progress: None,
        },
    }));

    let capabilities = ServerCapabilities {
        position_encoding,
        hover_provider,
        completion_provider,
        signature_help_provider,
        definition_provider,
        text_document_sync,
        document_symbol_provider: Some(OneOf::Left(true)),
        references_provider,
        diagnostic_provider,
        ..ServerCapabilities::default()
    };
    let server_capabilities = serde_json::to_value(capabilities).unwrap();
    let initialization_params = connection.initialize(server_capabilities)?;

    let params: InitializeParams = serde_json::from_value(initialization_params).unwrap();
    info!("Client initialization params: {:?}", params);
    let config = match get_root_config(&params) {
        Ok(cfg) => cfg,
        Err(e) => {
            let msg = format!("{e}. Please make corrections and restart asm-lsp.");
            send_notification(msg, MessageType::ERROR, &connection)?;
            // HACK: Sleep so our error message isn't immediately overwritten by
            // the LSP client informing the user that we exited with an error code
            sleep(Duration::from_secs(5));
            std::process::exit(1);
        }
    };
    if config == RootConfig::default() {
        let msg = "No .asm-lsp.toml config file found. Using default options.".to_string();
        send_notification(msg, MessageType::WARNING, &connection)?;
    }
    info!("Server Configuration: {:?}", config);

    let mut store = ServerStore::default();
    // Populate names to `Instruction`/`Register`/`Directive` maps
    for isa in config.effective_arches() {
        isa.setup_instructions(None, &mut store.names_to_info.instructions);
        isa.setup_registers(&mut store.names_to_info.registers);
    }

    for assembler in config.effective_assemblers() {
        assembler.setup_directives(&mut store.names_to_info.directives);
        if assembler == Assembler::Mars {
            Arch::Mips
                .setup_instructions(Some(Assembler::Mars), &mut store.names_to_info.instructions);
        }
    }

    // Use the maps we populated above to generate completion items
    store.completion_items.instructions = get_completes(
        &store.names_to_info.instructions,
        Some(CompletionItemKind::OPERATOR),
    );

    store.completion_items.registers = get_completes(
        &store.names_to_info.registers,
        Some(CompletionItemKind::VARIABLE),
    );

    store.completion_items.directives = get_completes(
        &store.names_to_info.directives,
        Some(CompletionItemKind::OPERATOR),
    );

    store.compile_commands = get_compile_cmds_from_file(&params).unwrap_or_default();
    if !store.compile_commands.is_empty() {
        info!("Loaded compile commands: {:?}", store.compile_commands);
    }
    store.include_dirs = get_include_dirs(&store.compile_commands);

    main_loop(&connection, &config, &store)?;

    // HACK: the `writer` thread of `connection` hangs on joining more often than
    // not. Need to investigate this further, but for now just skipping the join
    // (and thus allowing the process to exit) is fine
    // _io_threads.join()?;

    info!("Shutting down asm-lsp");
    Ok(())
}

fn main_loop(connection: &Connection, config: &RootConfig, store: &ServerStore) -> Result<()> {
    let mut doc_store = DocumentStore::new();

    info!("Starting asm-lsp loop...");
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    info!("Received shutdown request");
                    return Ok(());
                }
                handle_request(req, connection, config, &mut doc_store, store)?;
            }
            Message::Notification(notif) => {
                handle_notification(notif, connection, &mut doc_store, config, store)?;
            }
            Message::Response(resp) => warn!("Unexpected client response: {resp:?}"),
        }
    }
    Ok(())
}
