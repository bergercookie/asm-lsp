use crate::RegenerateDocs;

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, anyhow};

impl RegenerateDocs {
    pub fn run() -> Result<()> {
        // TODO: Ideally this would just call into the logic we have set up in the
        // asm-lsp-parsing package.
        let xtask_path: PathBuf = env!("CARGO_MANIFEST_DIR").into();
        let root_path = xtask_path.parent().unwrap();
        Command::new("cargo")
            .args(["build", "--workspace", "--all-targets", "--release"])
            .current_dir(root_path)
            .status()?;
        gen_opcodes(root_path)?;
        gen_registers(root_path)?;
        gen_directives(root_path)?;
        Ok(())
    }
}

fn gen_opcodes(root_path: &Path) -> Result<()> {
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/opcodes/x86.xml",
            "-o",
            "asm-lsp/serialized/opcodes/x86",
        ])
        .arg("--doc-type")
        .arg("instruction")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized x86 opcodes"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/opcodes/x86_64.xml",
            "-o",
            "asm-lsp/serialized/opcodes/x86_64",
        ])
        .arg("--doc-type")
        .arg("instruction")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized x86-64 opcodes"))?;
    // TODO: get the official arm32 opcode files
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/opcodes/ARM/",
            "-o",
            "asm-lsp/serialized/opcodes/arm",
        ])
        .arg("--doc-type")
        .arg("instruction")
        .arg("--arch")
        .arg("arm")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized arm opcodes"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/opcodes/ARM/",
            "-o",
            "asm-lsp/serialized/opcodes/arm64",
        ])
        .arg("--doc-type")
        .arg("instruction")
        .arg("--arch")
        .arg("arm64")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized arm64 opcodes"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/opcodes/RISCV/",
            "-o",
            "asm-lsp/serialized/opcodes/riscv",
        ])
        .arg("--doc-type")
        .arg("instruction")
        .arg("--arch")
        .arg("riscv")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized riscv opcodes"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/opcodes/z80.xml",
            "-o",
            "asm-lsp/serialized/opcodes/z80",
        ])
        .arg("--doc-type")
        .arg("instruction")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized z80 opcodes"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/opcodes/6502.html",
            "-o",
            "asm-lsp/serialized/opcodes/6502",
        ])
        .arg("--doc-type")
        .arg("instruction")
        .arg("--arch")
        .arg("6502")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized 6502 opcodes"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/opcodes/power-isa.json",
            "-o",
            "asm-lsp/serialized/opcodes/power-isa",
        ])
        .arg("--doc-type")
        .arg("instruction")
        .arg("--arch")
        .arg("power-isa")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized power-isa opcodes"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/opcodes/avr.xml",
            "-o",
            "asm-lsp/serialized/opcodes/avr",
        ])
        .arg("--doc-type")
        .arg("instruction")
        .arg("--arch")
        .arg("AVR")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized avr opcodes"))?;

    Ok(())
}

fn gen_registers(root_path: &Path) -> Result<()> {
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/registers/x86.xml",
            "-o",
            "asm-lsp/serialized/registers/x86",
        ])
        .arg("--doc-type")
        .arg("register")
        .arg("--arch")
        .arg("x86")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized x86 registers"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/registers/x86_64.xml",
            "-o",
            "asm-lsp/serialized/registers/x86_64",
        ])
        .arg("--doc-type")
        .arg("register")
        .arg("--arch")
        .arg("x86-64")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized x86-64 registers"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/registers/arm.xml",
            "-o",
            "asm-lsp/serialized/registers/arm",
        ])
        .arg("--doc-type")
        .arg("register")
        .arg("--arch")
        .arg("arm")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized arm registers"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/registers/arm64.xml",
            "-o",
            "asm-lsp/serialized/registers/arm64",
        ])
        .arg("--doc-type")
        .arg("register")
        .arg("--arch")
        .arg("arm64")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized arm64 registers"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/registers/riscv.rst.txt",
            "-o",
            "asm-lsp/serialized/registers/riscv",
        ])
        .arg("--doc-type")
        .arg("register")
        .arg("--arch")
        .arg("riscv")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized riscv registers"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/registers/z80.xml",
            "-o",
            "asm-lsp/serialized/registers/z80",
        ])
        .arg("--doc-type")
        .arg("register")
        .arg("--arch")
        .arg("z80")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized z80 registers"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/registers/6502.xml",
            "-o",
            "asm-lsp/serialized/registers/6502",
        ])
        .arg("--doc-type")
        .arg("register")
        .arg("--arch")
        .arg("6502")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized 6502 registers"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/registers/power-isa.xml",
            "-o",
            "asm-lsp/serialized/registers/power-isa",
        ])
        .arg("--doc-type")
        .arg("register")
        .arg("--arch")
        .arg("power-isa")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized power-isa registers"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/registers/avr.xml",
            "-o",
            "asm-lsp/serialized/registers/avr",
        ])
        .arg("--doc-type")
        .arg("register")
        .arg("--arch")
        .arg("AVR")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized avr registers"))?;

    Ok(())
}

fn gen_directives(root_path: &Path) -> Result<()> {
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/directives/gas.xml",
            "-o",
            "asm-lsp/serialized/directives/gas",
        ])
        .arg("--doc-type")
        .arg("directive")
        .arg("--assembler")
        .arg("gas")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized gas directives"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/directives/masm.xml",
            "-o",
            "asm-lsp/serialized/directives/masm",
        ])
        .arg("--doc-type")
        .arg("directive")
        .arg("--assembler")
        .arg("masm")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized masm directives"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/directives/nasm.xml",
            "-o",
            "asm-lsp/serialized/directives/nasm",
        ])
        .arg("--doc-type")
        .arg("directive")
        .arg("--assembler")
        .arg("nasm")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized nasm directives"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/directives/ca65.html",
            "-o",
            "asm-lsp/serialized/directives/ca65",
        ])
        .arg("--doc-type")
        .arg("directive")
        .arg("--assembler")
        .arg("ca65")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized ca65 directives"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/directives/avr.xml",
            "-o",
            "asm-lsp/serialized/directives/avr",
        ])
        .arg("--doc-type")
        .arg("directive")
        .arg("--assembler")
        .arg("avr")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized avr directives"))?;
    Command::new("target/release/asm_docs_parsing")
        .args([
            "docs_store/directives/fasm.xml",
            "-o",
            "asm-lsp/serialized/directives/fasm",
        ])
        .arg("--doc-type")
        .arg("directive")
        .arg("--assembler")
        .arg("fasm")
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized fasm directives"))?;
    Ok(())
}
