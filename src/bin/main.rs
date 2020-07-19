use look_asm::*;

use log::{error, info, warn};
use std::collections::HashMap;

// main -------------------------------------------------------------------------------------------
pub fn main() -> anyhow::Result<()> {
    // initialisation -----------------------------------------------------------------------------
    env_logger::init();

    // Embed the codes in the binary
    let xml_conts_x86 = include_str!("../../opcodes/x86.xml");
    let xml_conts_x86_64 = include_str!("../../opcodes/x86_64.xml");

    let mut instruction_sets = HashMap::<Arch, InstructionSet>::new();

    info!("Populating instruction set -> x86...");
    instruction_sets.insert(Arch::X86, generate_instruction_set(&xml_conts_x86)?);
    info!("Populating instruction set -> x86_64...");
    instruction_sets.insert(Arch::X86, generate_instruction_set(&xml_conts_x86_64)?);
    Ok(())
}
