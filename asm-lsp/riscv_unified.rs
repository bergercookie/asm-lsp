// RISC-V Unified Database Conversion Layer
// This module handles conversion between riscv-unified-db format and asm-lsp's internal types

use serde::{Deserialize, Deserializer, Serialize};

use crate::types::{
    Arch, Instruction, InstructionForm, Operand, OperandType, Register, RegisterType, RegisterWidth,
};

/// RISC-V Unified Database Instruction Format
/// This represents both the actual riscv-unified-db format and legacy test format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedRiscvInstruction {
    #[serde(rename = "$schema")]
    pub schema: Option<String>,
    pub kind: Option<String>,
    pub name: String,
    pub long_name: Option<String>,
    pub description: String,
    #[serde(rename = "definedBy")]
    pub defined_by: Option<DefinedBy>,
    pub assembly: Option<String>,
    pub encoding: Option<Encoding>,
    pub access: Option<Access>,
    pub data_independent_timing: Option<bool>,
    pub pseudoinstructions: Option<Vec<Pseudoinstruction>>,
    /// Operation field - handles both "operation" and "operation()" formats
    pub operation: Option<String>,

    /// Sail field - handles both "sail" and "sail()" formats
    pub sail: Option<String>,
    // Legacy fields for backward compatibility with test data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opcode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pseudo_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isa: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operands: Option<Vec<UnifiedOperand>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinedBy {
    /// Extension definition - can be simple or complex (allOf/oneOf)
    pub extension: Option<Extension>,

    /// For allOf structures - multiple conditions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allOf: Option<Vec<DefinedByCondition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinedByCondition {
    #[serde(flatten)]
    pub condition: DefinedByConditionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DefinedByConditionType {
    Extension(Extension),
    Xlen(XlenCondition),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XlenCondition {
    pub xlen: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extension {
    /// Extension name - can be a simple string or part of oneOf structure
    #[serde(alias = "name")]
    pub name: String,

    /// For oneOf structures - alternative extension names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oneOf: Option<Vec<ExtensionName>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionName {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encoding {
    #[serde(rename = "match")]
    pub match_pattern: String,
    pub variables: Vec<Variable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Access {
    pub s: String,
    pub u: String,
    pub vs: String,
    pub vu: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pseudoinstruction {
    pub when: String,
    pub to: String,
}

/// Legacy operand format for backward compatibility with test data
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
    // Extract extension information
    let _extension_name = unified
        .defined_by
        .as_ref()
        .and_then(|db| db.extension.as_ref().map(|ext| ext.name.clone()))
        .unwrap_or_else(|| "I".to_string());

    // Create assembly template from assembly field, format field (legacy), or construct from name
    let asm_template = if let Some(assembly) = &unified.assembly {
        assembly.replace(",", " ")
    } else if let Some(format) = &unified.format {
        format.to_string()
    } else {
        unified.name.to_lowercase()
    };

    let mut instruction = Instruction {
        name: unified.name.to_lowercase(),
        summary: unified.description.clone(),
        forms: vec![],
        asm_templates: vec![format!("{}", asm_template)],
        aliases: vec![],
        url: None, // URL not available in unified format
        arch: Arch::RISCV,
    };

    // Convert operands from encoding variables (new format) or operands field (legacy format)
    let mut operands = Vec::new();

    if let Some(encoding) = &unified.encoding {
        // New format: parse from encoding variables
        for var in &encoding.variables {
            let operand_type = match var.name.as_str() {
                "xd" | "xs1" | "xs2" | "xs3" | "rs1" | "rs2" | "rs3" | "rd" => OperandType::r32,
                "imm" => OperandType::imm32,
                _ => OperandType::_1, // Fallback
            };

            // Determine input/output based on variable name
            let is_output = var.name.starts_with("xd") || var.name == "rd";
            let is_input = !is_output || var.name.contains("imm");

            operands.push(Operand {
                type_: operand_type,
                input: Some(is_input),
                output: Some(is_output),
                extended_size: None,
            });
        }
    } else if let Some(operands_data) = &unified.operands {
        // Legacy format: parse from operands field
        for op in operands_data {
            let operand_type = match op.name.as_str() {
                "rd" | "rs1" | "rs2" | "rs3" => OperandType::r32,
                "imm" => OperandType::imm32,
                _ => OperandType::_1, // Fallback
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

    // Handle pseudoinstructions as aliases
    if let Some(pseudoinstructions) = &unified.pseudoinstructions {
        for pseudo in pseudoinstructions {
            instruction.aliases.push(crate::types::InstructionAlias {
                title: pseudo.to.clone(),
                summary: format!("Pseudoinstruction for {}: {}", unified.name, pseudo.when),
                asm_templates: vec![pseudo.to.clone()],
            });
        }
    }

    // Handle legacy aliases field for backward compatibility
    if let Some(aliases) = &unified.aliases {
        for alias in aliases {
            instruction.aliases.push(crate::types::InstructionAlias {
                title: alias.clone(),
                summary: format!("Alias for {}", unified.name),
                asm_templates: vec![
                    unified
                        .format
                        .clone()
                        .unwrap_or_else(|| unified.name.to_lowercase()),
                ],
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
            schema: Some("inst_schema.json#".to_string()),
            kind: Some("instruction".to_string()),
            name: "addi".to_string(),
            long_name: Some("Add immediate".to_string()),
            description: "Add immediate".to_string(),
            defined_by: Some(DefinedBy {
                extension: Extension {
                    name: "I".to_string(),
                },
            }),
            assembly: Some("xd, xs1, imm".to_string()),
            encoding: Some(Encoding {
                match_pattern: "-----------------000-----0010011".to_string(),
                variables: vec![
                    Variable {
                        name: "imm".to_string(),
                        location: "31-20".to_string(),
                    },
                    Variable {
                        name: "xs1".to_string(),
                        location: "19-15".to_string(),
                    },
                    Variable {
                        name: "xd".to_string(),
                        location: "11-7".to_string(),
                    },
                ],
            }),
            access: Some(Access {
                s: "always".to_string(),
                u: "always".to_string(),
                vs: "always".to_string(),
                vu: "always".to_string(),
            }),
            data_independent_timing: Some(true),
            pseudoinstructions: Some(vec![Pseudoinstruction {
                when: "imm == 0".to_string(),
                to: "mv xd,xs1".to_string(),
            }]),
            operation: Some("X[xd] = X[xs1] + $signed(imm)".to_string()),
            sail: None,
            // Legacy fields
            format: None,
            opcode: None,
            pseudo_code: None,
            extensions: None,
            isa: None,
            url: None,
            aliases: None,
            operands: None,
        };

        let instruction = convert_unified_instruction(&unified_instr);
        assert_eq!(instruction.name, "addi");
        assert_eq!(instruction.arch, Arch::RISCV);
        assert_eq!(instruction.asm_templates.len(), 1);
        assert_eq!(instruction.forms.len(), 1);
        // Check that pseudoinstructions were converted to aliases
        assert_eq!(instruction.aliases.len(), 1);
        assert_eq!(instruction.aliases[0].title, "mv xd,xs1");
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
        // Note: The real riscv-unified-db format uses individual files for each instruction
        // For testing, we'll use a list format that matches our expected input
        let yaml_data = r#"
- $schema: "inst_schema.json#"
  kind: instruction
  name: addi
  long_name: Add immediate
  description: Adds an immediate value to the value in xs1, and store the result in xd
  definedBy:
    extension:
      name: I
  assembly: xd, xs1, imm
  encoding:
    match: "-----------------000-----0010011"
    variables:
      - name: imm
        location: 31-20
      - name: xs1
        location: 19-15
      - name: xd
        location: 11-7
  access:
    s: always
    u: always
    vs: always
    vu: always
  data_independent_timing: true
"#;

        let result = load_unified_instructions_from_yaml(yaml_data);
        if let Err(e) = &result {
            eprintln!("YAML parsing error: {}", e);
        }
        assert!(result.is_ok());
        let instructions = result.unwrap();
        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0].name, "addi");
        assert_eq!(instructions[0].asm_templates.len(), 1);
        assert_eq!(instructions[0].forms.len(), 1);
        // Check that operands were parsed
        assert_eq!(instructions[0].forms[0].operands.len(), 3);
    }

    /// Helper function for testing YAML parsing directly
    fn load_unified_instructions_from_yaml(
        yaml_data: &str,
    ) -> Result<Vec<Instruction>, Box<dyn std::error::Error>> {
        let unified_instructions: Vec<UnifiedRiscvInstruction> = serde_saphyr::from_str(yaml_data)?;

        let mut instructions = Vec::new();
        for unified_instr in unified_instructions {
            instructions.push(convert_unified_instruction(&unified_instr));
        }

        Ok(instructions)
    }
}
use std::fs;

fn main() {
    let yaml_content =
        fs::read_to_string("docs_store/riscv-unified-db/riscv_yaml/Zaamo/amoadd.d.yaml").unwrap();

    // Try parsing as raw YAML first
}
