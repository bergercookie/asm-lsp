// ARM/ARM64 Architecture Parser
// This module handles all ARM and ARM64 specific parsing

use std::path::PathBuf;
use anyhow::Result;
use crate::types::Instruction;

/// Parse ARM/ARM64 instructions from XML files
pub fn parse_arm_instructions(docs_path: &PathBuf) -> Result<Vec<Instruction>> {
    // This will contain the ARM parsing logic moved from parser.rs
    // For now, we'll keep a stub that calls the original function
    super::parser::populate_arm_instructions(docs_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_arm_instruction_parsing() {
        // Test with a non-existent path - should fail gracefully
        let path = PathBuf::from("nonexistent.xml");
        let result = parse_arm_instructions(&path);
        assert!(result.is_err()); // Expected to fail with non-existent file
    }
}