use std::path::PathBuf;

use ::asm_lsp::x86_parser::{populate_instructions, populate_registers};
use asm_lsp::populate_directives;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, clap::ValueEnum)]
enum DocType {
    Instruction,
    Register,
    Directive,
}

#[derive(Parser, Debug)]
struct SerializeDocs {
    #[clap(required = true, help = "Path to the xml docs file")]
    input_path: PathBuf,
    #[arg(long, short, help = "Path to store the output file")]
    output_path: Option<PathBuf>,
    #[arg(
        long,
        short,
        required = true,
        help = "Serialize input as set of `instruction`, `register`, or `directive` items"
    )]
    doc_type: DocType,
}

#[derive(Subcommand)]
#[command(about = "Parse and serialize assembly-related documentation")]
enum Commands {
    SerializeDocs(SerializeDocs),
}

fn run(opts: &SerializeDocs) -> Result<()> {
    match opts.doc_type {
        DocType::Instruction => {
            let path = opts.input_path.canonicalize()?;
            let conts = std::fs::read_to_string(path)?;
            let instrs = populate_instructions(&conts)?;
            // For now we'll assume all instructions out of a single file share
            // a common architecture
            let arch = if let Some(instr) = instrs.first() {
                if let Some(arch) = instr.arch {
                    arch
                } else {
                    return Err(anyhow!(
                        "Failed to determine architecture -- Empty 'arch' field"
                    ));
                }
            } else {
                return Err(anyhow!(
                    "Failed to determine architecture -- Zero instructions read in"
                ));
            };
            let serialized = bincode::serialize(&instrs)?;
            let output_path: PathBuf = if let Some(ref path) = opts.output_path {
                path.to_owned()
            } else {
                format!("{arch}_instrs").into()
            };
            std::fs::write(output_path, serialized)?;
        }
        DocType::Register => {
            let path = opts.input_path.canonicalize()?;
            let conts = std::fs::read_to_string(path)?;
            let regs = populate_registers(&conts)?;
            // For now we'll assume all registers out of a single file share
            // a common architecture
            let arch = if let Some(instr) = regs.first() {
                if let Some(arch) = instr.arch {
                    arch
                } else {
                    return Err(anyhow!(
                        "Failed to determine architecture -- Empty 'arch' field"
                    ));
                }
            } else {
                return Err(anyhow!(
                    "Failed to determine architecture -- Zero registers read in"
                ));
            };
            let serialized = bincode::serialize(&regs)?;
            let output_path: PathBuf = if let Some(ref path) = opts.output_path {
                path.to_owned()
            } else {
                format!("{arch}_regs").into()
            };
            std::fs::write(output_path, serialized)?;
        }
        DocType::Directive => {
            let path = opts.input_path.canonicalize()?;
            let conts = std::fs::read_to_string(path)?;
            let directives = populate_directives(&conts)?;
            // For now we'll assume all directives out of a single file share
            // a common assembler
            let assembler = if let Some(instr) = directives.first() {
                if let Some(assem) = instr.assembler {
                    assem
                } else {
                    return Err(anyhow!(
                        "Failed to determine architecture -- Empty 'assembler' field"
                    ));
                }
            } else {
                return Err(anyhow!(
                    "Failed to determine assembler -- Zero directives read in"
                ));
            };
            let serialized = bincode::serialize(&directives)?;
            let output_path: PathBuf = if let Some(ref path) = opts.output_path {
                path.to_owned()
            } else {
                format!("{assembler}_directives").into()
            };
            std::fs::write(output_path, serialized)?;
        }
    }
    Ok(())
}

fn main() {
    let opts = SerializeDocs::parse();
    if let Err(e) = run(&opts) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
