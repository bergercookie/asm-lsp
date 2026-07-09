// RISC-V Architecture Parser
// This module handles all RISC-V specific parsing

use std::path::PathBuf;

use crate::{
    riscv_unified::{load_unified_instructions, load_unified_registers},
    types::{Instruction, Register},
};
use anyhow::Result;

/// Parse RISC-V registers from RST format (legacy)
/// This function is kept for backward compatibility
pub fn parse_riscv_registers_rst(rst_contents: &str) -> Result<Vec<Register>> {
    // This will contain the legacy RST parsing logic moved from parser.rs
    // For now, we'll keep a stub that calls the original function
    super::parser::populate_riscv_registers(rst_contents)
}

/// Parse RISC-V instructions from RST files (legacy)
/// This function is kept for backward compatibility
pub fn parse_riscv_instructions_rst(docs_path: &PathBuf) -> Result<Vec<Instruction>> {
    // This will contain the legacy RST parsing logic moved from parser.rs
    // For now, we'll keep a stub that calls the original function
    super::parser::populate_riscv_instructions(docs_path)
}

/// Parse RISC-V instructions from unified database format (preferred)
pub fn parse_riscv_instructions_unified(path: &str) -> Result<Vec<Instruction>> {
    load_unified_instructions(path).map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// Parse RISC-V registers from unified database format (preferred)
pub fn parse_riscv_registers_unified(path: &str) -> Result<Vec<Register>> {
    load_unified_registers(path).map_err(|e| anyhow::anyhow!(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_instruction_parsing() {
        let result = parse_riscv_instructions_unified("../asm-lsp/test_riscv_instructions.json");
        assert!(result.is_ok());

        let instructions = result.unwrap();
        assert_eq!(instructions.len(), 2);
        assert_eq!(instructions[0].name, "addi");
    }

    #[test]
    fn test_unified_register_parsing() {
        let result = parse_riscv_registers_unified("../asm-lsp/test_riscv_registers.json");
        assert!(result.is_ok());

        let registers = result.unwrap();
        assert_eq!(registers.len(), 3);
        assert_eq!(registers[0].name, "x0");
    }
}
