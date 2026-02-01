# RISC-V Unified Database

This directory contains RISC-V instruction and register data imported from the [riscv-unified-db](https://github.com/riscv-unified-db/riscv-unified-db) project.

## Source

The data is sourced from the canonical RISC-V Unified Database repository:
- **Repository**: https://github.com/riscv-unified-db/riscv-unified-db
- **License**: BSD-3-Clause-Clear (see LICENSE.txt)
- **Format**: YAML files containing comprehensive RISC-V instruction and register specifications

## Files

- `riscv_yaml/`: Individual YAML files for each RISC-V instruction
- `riscv_full_consolidated.json`: Consolidated JSON format generated from the YAML files
- `LICENSE.txt`: License information for the imported data

## Usage

The asm-lsp project uses this data to provide comprehensive RISC-V assembly language support, including:
- Instruction parsing and validation
- Register information
- Operand encoding details
- Architecture-specific features

## Updates

To update the RISC-V data:
1. Pull the latest changes from the riscv-unified-db repository
2. Run the YAML processing tool to regenerate the consolidated JSON
3. Commit the updated files to this repository
