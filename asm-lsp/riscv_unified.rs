// RISC-V Unified Database Conversion Layer
// This module handles conversion between riscv-unified-db format and asm-lsp's internal types

use serde::{Deserialize, Serialize};

use crate::types::{
    Arch, Instruction, InstructionForm, Operand, OperandType, Register, RegisterType, RegisterWidth,
};

/// RISC-V Unified Database Instruction Format
/// This represents the typical structure found in riscv-unified-db
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedRiscvInstruction {
    pub name: String,
    pub description: String,
    pub format: String,
    pub opcode: String,
    pub pseudo_code: Option<String>,
    pub extensions: Vec<String>,
    pub operands: Vec<UnifiedOperand>,
    pub isa: Option<String>,
    pub url: Option<String>,
    pub aliases: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedOperand {
    pub name: String,
    pub description: String,
    pub direction: String, // "input", "output", or "inout"
    pub width: Option<u32>,
}

/// RISC-V Unified Database Register Format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedRiscvRegister {
    pub name: String,
    pub abi_name: Option<String>,
    pub description: String,
    pub reg_type: String,
    pub width: u32,
    pub number: Option<u32>,
    pub saved_by: Option<String>,
    pub url: Option<String>,
}

/// Convert a unified RISC-V instruction to asm-lsp's Instruction format
pub fn convert_unified_instruction(unified: &UnifiedRiscvInstruction) -> Instruction {
    let mut instruction = Instruction {
        name: unified.name.to_lowercase(),
        summary: unified.description.clone(),
        forms: vec![],
        asm_templates: vec![unified.format.clone()],
        aliases: vec![],
        url: unified.url.clone(),
        arch: Arch::RISCV,
    };

    // Convert operands to asm-lsp format
    let mut operands = Vec::new();
    for op in &unified.operands {
        let operand_type = match op.name.as_str() {
            "rd" | "rs1" | "rs2" | "rs3" => OperandType::r32, // Default to 32-bit, will be adjusted
            "imm" => OperandType::imm32,                      // Default immediate type
            _ => OperandType::_1,                             // Fallback
        };

        let input = match op.direction.as_str() {
            "input" | "inout" => Some(true),
            "output" => Some(false),
            _ => None,
        };

        let output = match op.direction.as_str() {
            "output" | "inout" => Some(true),
            "input" => Some(false),
            _ => None,
        };

        operands.push(Operand {
            type_: operand_type,
            input,
            output,
            extended_size: None,
        });
    }

    // Create instruction form
    let form = InstructionForm {
        gas_name: None,
        go_name: None,
        mmx_mode: None,
        xmm_mode: None,
        cancelling_inputs: None,
        nacl_version: None,
        nacl_zero_extends_outputs: None,
        z80_name: None,
        z80_form: None,
        z80_opcode: None,
        z80_timing: None,
        avr_mneumonic: None,
        avr_summary: None,
        avr_version: None,
        avr_timing: None,
        avr_status_register: None,
        isa: None,
        operands,
        urls: vec![],
    };

    instruction.forms.push(form);

    // Handle aliases if present
    if let Some(aliases) = &unified.aliases {
        for alias in aliases {
            instruction.aliases.push(crate::types::InstructionAlias {
                title: alias.clone(),
                summary: format!("Alias for {}", unified.name),
                asm_templates: vec![unified.format.clone()],
            });
        }
    }

    instruction
}

/// Convert a unified RISC-V register to asm-lsp's Register format
pub fn convert_unified_register(unified: &UnifiedRiscvRegister) -> Register {
    let reg_type = match unified.reg_type.as_str() {
        "integer" => Some(RegisterType::GeneralPurpose),
        "floating_point" => Some(RegisterType::FloatingPoint),
        "control" => Some(RegisterType::Control),
        _ => None,
    };

    let width = match unified.width {
        32 => Some(RegisterWidth::Bits32),
        64 => Some(RegisterWidth::Bits64),
        128 => Some(RegisterWidth::Bits128),
        _ => None,
    };

    let mut description = unified.description.clone();
    if let Some(saved_by) = &unified.saved_by {
        description = format!("{}\n{} saved", description, saved_by);
    }

    Register {
        name: unified.name.to_lowercase(),
        description: Some(description),
        reg_type,
        width,
        flag_info: vec![],
        arch: Arch::RISCV,
        url: unified.url.clone(),
    }
}

/// Load and convert RISC-V instructions from unified database format
pub fn load_unified_instructions(
    path: &str,
) -> Result<Vec<Instruction>, Box<dyn std::error::Error>> {
    let data = std::fs::read_to_string(path)?;
    
    // Try JSON format first
    let unified_instructions: Vec<UnifiedRiscvInstruction> = match serde_json::from_str(&data) {
        Ok(instrs) => instrs,
        Err(_) => {
            // If JSON fails, try YAML format
            serde_saphyr::from_str(&data)?
        }
    };

    let mut instructions = Vec::new();
    for unified_instr in unified_instructions {
        instructions.push(convert_unified_instruction(&unified_instr));
    }

    Ok(instructions)
}

/// Load and convert RISC-V registers from unified database format
pub fn load_unified_registers(path: &str) -> Result<Vec<Register>, Box<dyn std::error::Error>> {
    let data = std::fs::read_to_string(path)?;
    
    // Try JSON format first
    let unified_registers: Vec<UnifiedRiscvRegister> = match serde_json::from_str(&data) {
        Ok(regs) => regs,
        Err(_) => {
            // If JSON fails, try YAML format
            serde_saphyr::from_str(&data)?
        }
    };

    let mut registers = Vec::new();
    for unified_reg in unified_registers {
        registers.push(convert_unified_register(&unified_reg));
    }

    Ok(registers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_conversion() {
        let unified_instr = UnifiedRiscvInstruction {
            name: "ADDI".to_string(),
            description: "Add immediate".to_string(),
            format: "addi rd, rs1, imm".to_string(),
            opcode: "0010011".to_string(),
            pseudo_code: Some("x[rd] = x[rs1] + sext(immediate)".to_string()),
            extensions: vec!["I".to_string()],
            operands: vec![
                UnifiedOperand {
                    name: "rd".to_string(),
                    description: "Destination register".to_string(),
                    direction: "output".to_string(),
                    width: Some(32),
                },
                UnifiedOperand {
                    name: "rs1".to_string(),
                    description: "Source register".to_string(),
                    direction: "input".to_string(),
                    width: Some(32),
                },
                UnifiedOperand {
                    name: "imm".to_string(),
                    description: "Immediate value".to_string(),
                    direction: "input".to_string(),
                    width: Some(12),
                },
            ],
            isa: Some("RV32I".to_string()),
            url: Some("https://riscv.org/specs/".to_string()),
            aliases: Some(vec!["addi_alias".to_string()]),
        };

        let instruction = convert_unified_instruction(&unified_instr);
        assert_eq!(instruction.name, "addi");
        assert_eq!(instruction.arch, Arch::RISCV);
        assert_eq!(instruction.asm_templates.len(), 1);
        assert_eq!(instruction.forms.len(), 1);
        assert_eq!(instruction.aliases.len(), 1);
    }

    #[test]
    fn test_register_conversion() {
        let unified_reg = UnifiedRiscvRegister {
            name: "x0".to_string(),
            abi_name: Some("zero".to_string()),
            description: "Hard-wired zero".to_string(),
            reg_type: "integer".to_string(),
            width: 64,
            number: Some(0),
            saved_by: None,
            url: Some("https://riscv.org/specs/".to_string()),
        };

        let register = convert_unified_register(&unified_reg);
        assert_eq!(register.name, "x0");
        assert_eq!(register.arch, Arch::RISCV);
        assert!(register.description.is_some());
        assert_eq!(register.reg_type, Some(RegisterType::GeneralPurpose));
    }

    #[test]
    fn test_yaml_parsing() {
        let yaml_data = r#"
- name: ADDI
  description: Add immediate
  format: "addi rd, rs1, imm"
  opcode: "0010011"
  extensions:
    - I
  operands:
    - name: rd
      description: "Destination register"
      direction: "output"
      width: 32
"#;

        let result = load_unified_instructions_from_yaml(yaml_data);
        if let Err(e) = &result {
            eprintln!("YAML parsing error: {}", e);
        }
        assert!(result.is_ok());
        let instructions = result.unwrap();
        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0].name, "addi");
    }

    /// Helper function for testing YAML parsing directly
    fn load_unified_instructions_from_yaml(yaml_data: &str) -> Result<Vec<Instruction>, Box<dyn std::error::Error>> {
        let unified_instructions: Vec<UnifiedRiscvInstruction> = serde_saphyr::from_str(yaml_data)?;

        let mut instructions = Vec::new();
        for unified_instr in unified_instructions {
            instructions.push(convert_unified_instruction(&unified_instr));
        }

        Ok(instructions)
    }
}
