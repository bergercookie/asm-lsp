#! /bin/bash

# Regenerate binary data stores
# This is necessary whenever a serialized data structure (e.g. `Instruction`, `Register`,
# `Arch`, etc.) is modified in the source code

# opcode binaries
cargo run --release -- ../docs_store/opcodes/raw/x86.xml -o ../asm-lsp/serialized/opcodes/x86 --doc-type instruction
cargo run --release -- ../docs_store/opcodes/raw/x86_64.xml -o ../asm-lsp/serialized/opcodes/x86_64 --doc-type instruction
cargo run --release -- ../docs_store/opcodes/raw/ARM/ -o ../asm-lsp/serialized/opcodes/arm --doc-type instruction --arch arm
cargo run --release -- ../docs_store/opcodes/raw/RISCV/ -o ../asm-lsp/serialized/opcodes/riscv --doc-type instruction --arch riscv
cargo run --release -- ../docs_store/opcodes/raw/z80.xml -o ../asm-lsp/serialized/opcodes/z80 --doc-type instruction

# register binaries
cargo run --release -- ../docs_store/registers/raw/x86.xml -o ../asm-lsp/serialized/registers/x86 --doc-type register --arch x86
cargo run --release -- ../docs_store/registers/raw/x86_64.xml -o ../asm-lsp/serialized/registers/x86_64 --doc-type register --arch x86-64
cargo run --release -- ../docs_store/registers/raw/arm.xml -o ../asm-lsp/serialized/registers/arm --doc-type register --arch arm
cargo run --release -- ../docs_store/registers/raw/riscv.rst.txt -o ../asm-lsp/serialized/registers/riscv --doc-type register --arch riscv
cargo run --release -- ../docs_store/registers/raw/z80.xml -o ../asm-lsp/serialized/registers/z80 --doc-type register --arch z80

# directive binaries
cargo run --release -- ../docs_store/directives/raw/gas.xml -o ../asm-lsp/serialized/directives/gas --doc-type directive --assembler gas
cargo run --release -- ../docs_store/directives/raw/masm.xml -o ../asm-lsp/serialized/directives/masm --doc-type directive --assembler masm
cargo run --release -- ../docs_store/directives/raw/nasm.xml -o ../asm-lsp/serialized/directives/nasm --doc-type directive --assembler nasm
