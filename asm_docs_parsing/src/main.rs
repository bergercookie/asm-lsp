use std::path::PathBuf;

use ::asm_lsp::parser::{populate_instructions, populate_registers};
use asm_lsp::{
    parser::populate_arm_instructions, populate_directives, Arch, Directive, Instruction, Register,
};

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
    #[clap(
        required = true,
        help = "Path to the xml docs file or directory of docs files"
    )]
    input_path: PathBuf,
    #[arg(long, short, help = "Path to store the output file")]
    output_path: PathBuf,
    #[arg(
        long,
        short,
        required = true,
        help = "Serialize input as set of `instruction`, `register`, or `directive` items"
    )]
    doc_type: DocType,
    #[arg(
        long,
        short,
        help = "Architecture. Must be specified if `input_path` is a directory, ignored otherwise"
    )]
    arch: Option<Arch>,
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
            let instrs: Vec<Instruction>;
            match (path.is_dir(), opts.arch) {
                (true, Some(arch)) => {
                    if arch == Arch::ARM {
                        instrs = populate_arm_instructions(&opts.input_path)?;
                    } else {
                        return Err(anyhow!(
                            "Directory parsing for {arch} instructions is not supported"
                        ));
                    }
                }
                (true, None) => {
                    return Err(anyhow!(
                        "`Arch` argument must be supplied when `input_path` is a directory"
                    ));
                }
                (false, arch_in) => {
                    if arch_in.is_some() {
                        println!("WARNING: `Arch` argument is ignored when `input_path` isn't a directory");
                    }
                    let conts = std::fs::read_to_string(&path)?;
                    instrs = populate_instructions(&conts)?;
                }
            }
            if instrs.is_empty() {
                return Err(anyhow!("Zero instructions read in"));
            }
            let serialized = bincode::serialize(&instrs)?;
            std::fs::write(&opts.output_path, serialized)?;
        }
        DocType::Register => {
            let path = opts.input_path.canonicalize()?;
            let conts = std::fs::read_to_string(&path)?;
            let regs: Vec<Register> = match (path.is_dir(), opts.arch) {
                (true, _) => {
                    return Err(anyhow!("Directory parsing is not supported for registers"));
                }
                (false, arch_in) => {
                    if arch_in.is_some() {
                        println!("WARNING: `Arch` argument is ignored when `input_path` isn't a directory");
                    }
                    populate_registers(&conts)?
                }
            };
            if regs.is_empty() {
                return Err(anyhow!("Zero registers read in"));
            }
            let serialized = bincode::serialize(&regs)?;
            std::fs::write(&opts.output_path, serialized)?;
        }
        DocType::Directive => {
            let path = opts.input_path.canonicalize()?;
            let conts = std::fs::read_to_string(&path)?;
            let directives: Vec<Directive> = match (path.is_dir(), opts.arch) {
                (true, _) => {
                    return Err(anyhow!("Directory parsing is not supported for directives"));
                }
                (false, arch_in) => {
                    if arch_in.is_some() {
                        println!("WARNING: `Arch` argument is ignored when `input_path` isn't a directory");
                    }
                    populate_directives(&conts)?
                }
            };
            if directives.is_empty() {
                return Err(anyhow!("Zero directives read in"));
            }
            let serialized = bincode::serialize(&directives)?;
            std::fs::write(&opts.output_path, serialized)?;
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
