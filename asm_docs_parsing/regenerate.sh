#! /bin/bash

# Regenerate binary data stores
# This is necessary whenever a serialized data structure (e.g. `Instruction`, `Register`,
# `Arch`, etc.) is modified in the source code

# opcode binaries
cargo run --release -- ../docs_store/opcodes/raw/x86.xml -o ../docs_store/opcodes/serialized/x86 --doc-type instruction
cargo run --release -- ../docs_store/opcodes/raw/x86_64.xml -o ../docs_store/opcodes/serialized/x86_64 --doc-type instruction
cargo run --release -- ../docs_store/opcodes/raw/ARM/ -o ../docs_store/opcodes/serialized/arm --doc-type instruction --arch arm
cargo run --release -- ../docs_store/opcodes/raw/RISCV/ -o ../docs_store/opcodes/serialized/riscv --doc-type instruction --arch riscv
cargo run --release -- ../docs_store/opcodes/raw/z80.xml -o ../docs_store/opcodes/serialized/z80 --doc-type instruction

# register binaries
cargo run --release -- ../docs_store/registers/raw/x86.xml -o ../docs_store/registers/serialized/x86 --doc-type register --arch x86
cargo run --release -- ../docs_store/registers/raw/x86_64.xml -o ../docs_store/registers/serialized/x86_64 --doc-type register --arch x86-64
cargo run --release -- ../docs_store/registers/raw/arm.xml -o ../docs_store/registers/serialized/arm --doc-type register --arch arm
cargo run --release -- ../docs_store/registers/raw/riscv.rst.txt -o ../docs_store/registers/serialized/riscv --doc-type register --arch riscv
cargo run --release -- ../docs_store/registers/raw/z80.xml -o ../docs_store/registers/serialized/z80 --doc-type register --arch z80

# directive binaries
cargo run --release -- ../docs_store/directives/raw/gas.xml -o ../docs_store/directives/serialized/gas --doc-type directive --assembler gas
cargo run --release -- ../docs_store/directives/raw/masm.xml -o ../docs_store/directives/serialized/masm --doc-type directive --assembler masm
cargo run --release -- ../docs_store/directives/raw/nasm.xml -o ../docs_store/directives/serialized/nasm --doc-type directive --assembler nasm
