#! /bin/bash
set -x

# Regenerate binary data stores
# This is necessary whenever a serialized data structure (e.g. `Instruction`, `Register`,
# `Arch`, etc.) is modified in the source code

cargo build --release

# opcode binaries
../target/release/asm_docs_parsing ../docs_store/opcodes/x86.xml -o ../asm-lsp/serialized/opcodes/x86 --doc-type instruction
../target/release/asm_docs_parsing ../docs_store/opcodes/x86_64.xml -o ../asm-lsp/serialized/opcodes/x86_64 --doc-type instruction
# TODO: get the official arm32 opcode files
../target/release/asm_docs_parsing ../docs_store/opcodes/ARM/ -o ../asm-lsp/serialized/opcodes/arm --doc-type instruction --arch arm
../target/release/asm_docs_parsing ../docs_store/opcodes/ARM/ -o ../asm-lsp/serialized/opcodes/arm64 --doc-type instruction --arch arm64
../target/release/asm_docs_parsing ../docs_store/opcodes/RISCV/ -o ../asm-lsp/serialized/opcodes/riscv --doc-type instruction --arch riscv
../target/release/asm_docs_parsing ../docs_store/opcodes/z80.xml -o ../asm-lsp/serialized/opcodes/z80 --doc-type instruction
../target/release/asm_docs_parsing ../docs_store/opcodes/6502.html -o ../asm-lsp/serialized/opcodes/6502 --doc-type instruction --arch 6502
../target/release/asm_docs_parsing ../docs_store/opcodes/power-isa.json -o ../asm-lsp/serialized/opcodes/power-isa --doc-type instruction --arch power-isa

# register binaries
../target/release/asm_docs_parsing ../docs_store/registers/x86.xml -o ../asm-lsp/serialized/registers/x86 --doc-type register --arch x86
../target/release/asm_docs_parsing ../docs_store/registers/x86_64.xml -o ../asm-lsp/serialized/registers/x86_64 --doc-type register --arch x86-64
../target/release/asm_docs_parsing ../docs_store/registers/arm.xml -o ../asm-lsp/serialized/registers/arm --doc-type register --arch arm
../target/release/asm_docs_parsing ../docs_store/registers/arm64.xml -o ../asm-lsp/serialized/registers/arm64 --doc-type register --arch arm64
../target/release/asm_docs_parsing ../docs_store/registers/riscv.rst.txt -o ../asm-lsp/serialized/registers/riscv --doc-type register --arch riscv
../target/release/asm_docs_parsing ../docs_store/registers/z80.xml -o ../asm-lsp/serialized/registers/z80 --doc-type register --arch z80
../target/release/asm_docs_parsing ../docs_store/registers/6502.xml -o ../asm-lsp/serialized/registers/6502 --doc-type register --arch 6502
../target/release/asm_docs_parsing ../docs_store/registers/power-isa.xml -o ../asm-lsp/serialized/registers/power-isa --doc-type register --arch power-isa

# directive binaries
../target/release/asm_docs_parsing ../docs_store/directives/gas.xml -o ../asm-lsp/serialized/directives/gas --doc-type directive --assembler gas
../target/release/asm_docs_parsing ../docs_store/directives/masm.xml -o ../asm-lsp/serialized/directives/masm --doc-type directive --assembler masm
../target/release/asm_docs_parsing ../docs_store/directives/nasm.xml -o ../asm-lsp/serialized/directives/nasm --doc-type directive --assembler nasm
../target/release/asm_docs_parsing ../docs_store/directives/ca65.html -o ../asm-lsp/serialized/directives/ca65 --doc-type directive --assembler ca65
../target/release/asm_docs_parsing ../docs_store/directives/avr.xml -o ../asm-lsp/serialized/directives/avr --doc-type directive --assembler avr
