use std::path::PathBuf;

use ::asm_lsp::parser::{
    populate_6502_instructions, populate_arm_instructions, populate_ca65_directives,
    populate_gas_directives, populate_instructions, populate_masm_nasm_directives,
    populate_registers, populate_riscv_instructions, populate_riscv_registers,
};
use asm_lsp::{Arch, Assembler, Directive, Instruction, Register};

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
        help = "Architecture. Must be specified if `input_path` is a directory of opcode information, or register documentation. Ignored otherwise"
    )]
    arch: Option<Arch>,
    #[arg(
        long,
        help = "Assembler. Must be specified if parsing directive documentation, ignored otherwise"
    )]
    assembler: Option<Assembler>,
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
                (true, Some(arch)) => match arch {
                    Arch::ARM => {
                        instrs = populate_arm_instructions(&opts.input_path)?;
                    }
                    Arch::ARM64 => {
                        instrs = populate_arm_instructions(&opts.input_path)?;
                    }
                    Arch::RISCV => {
                        instrs = populate_riscv_instructions(&opts.input_path)?;
                    }
                    _ => {
                        return Err(anyhow!(
                            "Directory parsing for {arch} instructions is not supported"
                        ));
                    }
                },
                (true, None) => {
                    return Err(anyhow!(
                        "`Arch` argument must be supplied when `input_path` is a directory"
                    ));
                }
                (false, arch_in) => {
                    let conts = std::fs::read_to_string(&path)?;
                    match arch_in {
                        Some(Arch::MOS6502) => {
                            instrs = populate_6502_instructions(&conts)?;
                        }
                        _ => {
                            instrs = populate_instructions(&conts)?;
                        }
                    }
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
                (false, None) => {
                    return Err(anyhow!(
                        "`arch` argument is requred when parsing register documentation"
                    ));
                }
                (false, Some(arch_in)) => {
                    if arch_in == Arch::RISCV {
                        populate_riscv_registers(&conts)
                    } else {
                        populate_registers(&conts)?
                    }
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
            let directives: Vec<Directive> = match (path.is_dir(), opts.assembler) {
                (true, _) => {
                    return Err(anyhow!("Directory parsing is not supported for directives"));
                }
                (false, None) => {
                    return Err(anyhow!(
                        "`assembler` argument is requred when parsing directive documentation"
                    ));
                }
                (false, Some(assembler_in)) => match assembler_in {
                    Assembler::Gas | Assembler::Go => populate_gas_directives(&conts)?,
                    Assembler::Masm | Assembler::Nasm => populate_masm_nasm_directives(&conts)?,
                    Assembler::Ca65 => populate_ca65_directives(&conts)?,
                    Assembler::None => unreachable!(),
                },
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
