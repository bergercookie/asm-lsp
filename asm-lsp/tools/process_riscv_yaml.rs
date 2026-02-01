// Tool to process riscv-unified-db YAML files and create unified database files
// This tool converts individual YAML instruction files into a consolidated format
// that can be used by the asm-lsp build system

use std::{
    fs,
    path::{Path, PathBuf},
};
use serde_saphyr;
use anyhow::{Context, Result};

use asm_lsp::riscv_unified::UnifiedRiscvInstruction;

/// Process a directory of RISC-V YAML files and create a consolidated JSON file
pub fn process_riscv_directory(input_dir: &Path, output_file: &Path) -> Result<()> {
    println!("Processing RISC-V YAML files from: {}", input_dir.display());
    
    // Find all YAML files in the directory (recursively)
    let mut yaml_files = Vec::new();
    
    if input_dir.is_dir() {
        find_yaml_files_recursive(input_dir, &mut yaml_files)?;
    }
    
    println!("Found {} YAML files", yaml_files.len());
    
    // Process each YAML file
    let mut all_instructions = Vec::new();
    let mut successful = 0;
    let mut failed = 0;
    
    for (i, file_path) in yaml_files.iter().enumerate() {
        if i % 100 == 0 {
            println!("Processing file {}/{}...", i, yaml_files.len());
        }
        
        match process_single_yaml_file(file_path) {
            Ok(instruction) => {
                all_instructions.push(instruction);
                successful += 1;
            }
            Err(e) => {
                eprintln!("Failed to process {}: {}", file_path.display(), e);
                failed += 1;
            }
        }
    }
    
    println!("Processed {} files: {} successful, {} failed", 
             yaml_files.len(), successful, failed);
    
    // Write consolidated output
    let json_data = serde_json::to_string_pretty(&all_instructions)
        .context("Failed to serialize instructions to JSON")?;
    
    fs::write(output_file, json_data)
        .context("Failed to write output file")?;
    
    println!("Successfully wrote consolidated data to: {}", output_file.display());
    
    Ok(())
}

/// Recursively find all YAML files in a directory
fn find_yaml_files_recursive(dir: &Path, yaml_files: &mut Vec<PathBuf>) -> Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Recursively search subdirectories
                find_yaml_files_recursive(&path, yaml_files)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                // Add YAML files to the list
                yaml_files.push(path);
            }
        }
    }
    Ok(())
}

/// Process a single YAML file into our unified instruction format
fn process_single_yaml_file(file_path: &Path) -> Result<UnifiedRiscvInstruction> {
    let yaml_content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;
    
    // Parse the YAML into our unified format
    let mut instruction: UnifiedRiscvInstruction = serde_saphyr::from_str(&yaml_content)
        .with_context(|| format!("Failed to parse YAML: {}", file_path.display()))?;
    
    // Set the name to lowercase for consistency
    instruction.name = instruction.name.to_lowercase();
    
    // If no assembly field, try to construct from name
    if instruction.assembly.is_none() {
        instruction.assembly = Some(instruction.name.clone());
    }
    
    Ok(instruction)
}

/// Main function for command-line usage
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Usage: {} <input_directory> <output_file>", args[0]);
        std::process::exit(1);
    }
    
    let input_dir = PathBuf::from(&args[1]);
    let output_file = PathBuf::from(&args[2]);
    
    process_riscv_directory(&input_dir, &output_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_process_single_yaml_file() {
        let yaml_content = r#"
$schema: "inst_schema.json#"
kind: instruction
name: test_instr
long_name: Test Instruction
description: A test instruction
definedBy:
  extension:
    name: I
assembly: xd, xs1, xs2
encoding:
  match: "-----------------000-----0010011"
  variables:
    - name: xs2
      location: 24-20
    - name: xs1
      location: 19-15
    - name: xd
      location: 11-7
access:
  s: always
  u: always
  vs: always
  vu: always
"#;

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_instr.yaml");
        fs::write(&file_path, yaml_content).unwrap();

        let result = process_single_yaml_file(&file_path);
        assert!(result.is_ok());
        
        let instruction = result.unwrap();
        assert_eq!(instruction.name, "test_instr");
        assert_eq!(instruction.assembly, Some("xd, xs1, xs2".to_string()));
        assert!(instruction.encoding.is_some());
        assert_eq!(instruction.encoding.unwrap().variables.len(), 3);
    }
}