# AMD GPU ISA Support for asm-lsp

This document describes the plan, implementation steps, and technical details for adding AMD GPU ISA support to `asm-lsp-amd`, a fork of [asm-lsp](https://github.com/bergercookie/asm-lsp).

---

## Table of Contents

1. [Motivation](#motivation)
2. [Supported GPU Generations](#supported-gpu-generations)
3. [Architecture of asm-lsp](#architecture-of-asm-lsp)
4. [Plan](#plan)
5. [Step-by-Step Execution](#step-by-step-execution)
   - [Step 0: Seeding the Fork](#step-0-seeding-the-fork)
   - [Step 1: Generating AMD ISA Data](#step-1-generating-amd-isa-data)
   - [Step 2: Adding Rust Type Variants](#step-2-adding-rust-type-variants)
   - [Step 3: Adding the JSON Parser](#step-3-adding-the-json-parser)
   - [Step 4: Wiring the Doc Serializer](#step-4-wiring-the-doc-serializer)
   - [Step 5: Wiring xtask](#step-5-wiring-xtask)
   - [Step 6: Config Builder](#step-6-config-builder)
   - [Step 7: JSON Schema](#step-7-json-schema)
   - [Step 8: Running Serialization](#step-8-running-serialization)
   - [Step 9: Tests](#step-9-tests)
6. [Data Pipeline Details](#data-pipeline-details)
   - [llvm-tblgen JSON Format](#llvm-tblgen-json-format)
   - [Filtering Real Instructions](#filtering-real-instructions)
   - [AsmString Cleaning](#asmstring-cleaning)
   - [GFX950 Special Handling](#gfx950-special-handling)
   - [GFX1250 Merging Strategy](#gfx1250-merging-strategy)
   - [Register XML Format](#register-xml-format)
7. [Files Changed](#files-changed)
8. [Usage](#usage)
9. [Instruction Counts](#instruction-counts)
10. [Known Limitations and Future Work](#known-limitations-and-future-work)
11. [Gotchas and Debugging Notes](#gotchas-and-debugging-notes)

---

## Motivation

`asm-lsp` provides Language Server Protocol (LSP) features (hover, completion, go-to-definition, signature help) for assembly language files in editors like Neovim and VSCode. While it supports x86/x86-64, ARM, RISC-V, MIPS, and others, it had no support for AMD GPU ISAs.

AMD GPU assembly (AMDGPU ISA) is used in:
- **ROCm** GPU compute workloads (HIP, OpenCL kernels)
- **Tensile / rocBLAS / rocISA** — hand-tuned GPU assembly for matrix operations
- **LLVM MCA** analysis of AMD GPU shader code
- Debugging GPU shaders at the ISA level

This fork adds LSP support for the following GPU families:
- **GFX11**: RDNA 3 (Radeon RX 7000) and CDNA 2/3 (Instinct MI200/MI300)
- **GFX950**: CDNA4 (Instinct MI350), ISA version 9.5.0
- **GFX12**: RDNA 4 (Radeon RX 9000)
- **GFX1250**: CDNA 4 (Instinct MI400+), based on GFX12 with additional compute instructions

---

## Supported GPU Generations

| Config value | Arch variant | Hardware | LLVM generation tag |
|---|---|---|---|
| `amdgpu-gfx11` | `Arch::AmdgpuGfx11` | RDNA3 (RX 7000) / CDNA2-3 (MI200/MI300) | `_gfx11` suffix |
| `amdgpu-gfx950` | `Arch::AmdgpuGfx950` | CDNA4 (MI350) | `HasGFX950Insts` predicate |
| `amdgpu-gfx12` | `Arch::AmdgpuGfx12` | RDNA4 (RX 9000) | `_gfx12` suffix |
| `amdgpu-gfx1250` | `Arch::AmdgpuGfx1250` | CDNA4 (MI400) | `_gfx1250` suffix + GFX12 base |

---

## Architecture of asm-lsp

Before diving into implementation, here is how asm-lsp is structured:

```
asm-lsp-amd/
├── asm-lsp/                      # Core LSP server (library + binary)
│   ├── src/lib.rs                # Re-exports, top-level
│   ├── types.rs                  # Arch enum, Instruction/Register/Directive structs
│   ├── parser.rs                 # Parsers: XML, JSON, HTML, RST → Vec<Instruction>
│   ├── handle.rs                 # LSP request handlers (hover, completion, ...)
│   ├── lsp.rs                    # Core LSP logic (completions, hover text generation)
│   ├── config_builder.rs         # Interactive config generator
│   ├── test.rs                   # Integration tests
│   └── serialized/
│       ├── opcodes/              # Bincode-serialized instruction sets (one file per arch)
│       └── registers/            # Bincode-serialized register sets
├── asm_docs_parsing/             # CLI tool: source docs → bincode
│   └── src/main.rs
├── docs_store/                   # Raw documentation source files
│   ├── opcodes/                  # XML, JSON, HTML, RST files per architecture
│   └── registers/                # XML files per architecture
├── xtask/                        # Build automation (regenerate-docs command)
│   └── src/regnerate_docs.rs
├── scripts/                      # (new) Data extraction scripts
│   └── gen_amdgpu_docs.py
└── asm-lsp_config_schema.json    # JSON schema for .asm-lsp.toml
```

### Data flow

```
LLVM TableGen (.td files)
        │
        │ llvm-tblgen --dump-json
        ▼
/tmp/amdgpu_all.json  (~450 MB)
        │
        │ scripts/gen_amdgpu_docs.py
        ▼
docs_store/opcodes/amdgpu-gfx{11,950,12,1250}.json
docs_store/registers/amdgpu.xml
        │
        │ cargo xtask regenerate-docs
        │ (calls asm_docs_parsing binary for each arch)
        ▼
asm-lsp/serialized/opcodes/amdgpu-gfx{11,950,12,1250}   (bincode)
asm-lsp/serialized/registers/amdgpu                       (bincode)
        │
        │ include_bytes!() at compile time
        ▼
LSP server runtime (hover, completion, ...)
```

### Lookup mechanism

Instructions are stored in a `HashMap<(Arch, String), Instruction>`. When the user hovers over `v_add_f32` in a file configured for `amdgpu-gfx12`, the server looks up `(Arch::AmdgpuGfx12, "v_add_f32")` and returns the hover text.

### Configuration

The user (or editor integration) declares which GPU generation to use in `.asm-lsp.toml`:

```toml
[default_config]
assembler = "gas"
instruction_set = "amdgpu-gfx12"
```

Different subdirectories can target different generations:

```toml
[[projects]]
path = "src/rdna3_kernels"
assembler = "gas"
instruction_set = "amdgpu-gfx11"

[[projects]]
path = "src/cdna4_mfma"
assembler = "gas"
instruction_set = "amdgpu-gfx1250"
```

---

## Plan

The implementation was planned in 9 steps:

1. **Seed the fork** — copy reference `asm-lsp` into `asm-lsp-amd`
2. **Generate AMD ISA data** — extract from LLVM TableGen via `llvm-tblgen --dump-json`, process with Python script
3. **Add Rust type variants** — extend `Arch` enum in `types.rs`, add `setup_registers/instructions` match arms and `Display` impl
4. **Add JSON parser** — `populate_amdgpu_instructions(arch, json_str)` in `parser.rs`
5. **Wire the doc serializer** — import + match arms in `asm_docs_parsing/src/main.rs`
6. **Wire xtask** — add generation commands in `xtask/src/regnerate_docs.rs`
7. **Config builder** — extend `ARCH_LIST` in `config_builder.rs`
8. **JSON schema** — add 4 new enum values to `asm-lsp_config_schema.json`
9. **Tests** — add serialization freshness tests in `test.rs`

---

## Step-by-Step Execution

### Step 0: Seeding the Fork

The `asm-lsp-amd` directory was empty (just a `.git`). Copy the reference repo:

```bash
cp -r /home/thomas/projects/asm-lsp/. /home/thomas/projects/asm-lsp-amd/
```

This gives us the full codebase to modify without touching the upstream repo.

---

### Step 1: Generating AMD ISA Data

#### 1a. Run `llvm-tblgen --dump-json`

The LLVM project contains machine-readable descriptions of every AMDGPU instruction in TableGen (`.td`) files. `llvm-tblgen --dump-json` compiles these into a single JSON file containing all records.

```bash
TBLGEN=/home/thomas/projects/rocMLIR/build/external/llvm-project/llvm/bin/llvm-tblgen
LLVM_AMDGPU=/home/thomas/projects/rocMLIR/external/llvm-project/llvm/lib/Target/AMDGPU
LLVM_INC=/home/thomas/projects/rocMLIR/external/llvm-project/llvm/include

$TBLGEN --dump-json \
  -I $LLVM_INC \
  -I $LLVM_AMDGPU \
  $LLVM_AMDGPU/AMDGPU.td \
  -o /tmp/amdgpu_all.json
```

> **Important**: Use the `llvm-tblgen` binary built from the **same LLVM checkout** as the `.td` source files. A version mismatch will cause parse errors. In this repo we use the binary from the rocMLIR build tree (LLVM 22 / AOMP 18).

The output is ~450 MB and contains ~45,923 `Instruction` records.

#### 1b. The `gen_amdgpu_docs.py` script

Located at `scripts/gen_amdgpu_docs.py`. Run from the repo root:

```bash
python3 scripts/gen_amdgpu_docs.py /tmp/amdgpu_all.json .
```

Outputs:
- `docs_store/opcodes/amdgpu-gfx11.json`
- `docs_store/opcodes/amdgpu-gfx950.json`
- `docs_store/opcodes/amdgpu-gfx12.json`
- `docs_store/opcodes/amdgpu-gfx1250.json`
- `docs_store/registers/amdgpu.xml`

The script is detailed in the [Data Pipeline Details](#data-pipeline-details) section.

---

### Step 2: Adding Rust Type Variants

**File**: `asm-lsp/types.rs`

#### 2a. Extend the `Arch` enum

Added four variants after `Arch::Mips`:

```rust
#[strum(serialize = "amdgpu-gfx11")]
#[serde(rename = "amdgpu-gfx11")]
AmdgpuGfx11,
#[strum(serialize = "amdgpu-gfx950")]
#[serde(rename = "amdgpu-gfx950")]
AmdgpuGfx950,
#[strum(serialize = "amdgpu-gfx12")]
#[serde(rename = "amdgpu-gfx12")]
AmdgpuGfx12,
#[strum(serialize = "amdgpu-gfx1250")]
#[serde(rename = "amdgpu-gfx1250")]
AmdgpuGfx1250,
```

The `#[strum(serialize = "...")]` attribute controls the string that `Arch::from_str()` and `to_string()` use — these must match the register XML's `InstructionSet name` attribute and the config TOML's `instruction_set` field.

#### 2b. Add `setup_registers()` match arms

All four AMD generations share the same physical register file (VGPR, SGPR, etc.), so they all load the same serialized register data:

```rust
Self::AmdgpuGfx11
| Self::AmdgpuGfx950
| Self::AmdgpuGfx12
| Self::AmdgpuGfx1250 => {
    load_registers_with_path!(self, "serialized/registers/amdgpu");
}
```

The `self` here is the variant-specific arch, so the register map will be keyed as `(AmdgpuGfx12, "v0")`, `(AmdgpuGfx11, "v0")`, etc., allowing per-generation hover even for registers.

#### 2c. Add `setup_instructions()` match arms

Each generation has its own serialized opcode file:

```rust
Self::AmdgpuGfx11 => {
    load_instructions_with_path!(Self::AmdgpuGfx11, "serialized/opcodes/amdgpu-gfx11");
}
Self::AmdgpuGfx950 => {
    load_instructions_with_path!(Self::AmdgpuGfx950, "serialized/opcodes/amdgpu-gfx950");
}
Self::AmdgpuGfx12 => {
    load_instructions_with_path!(Self::AmdgpuGfx12, "serialized/opcodes/amdgpu-gfx12");
}
Self::AmdgpuGfx1250 => {
    load_instructions_with_path!(Self::AmdgpuGfx1250, "serialized/opcodes/amdgpu-gfx1250");
}
```

#### 2d. Add `Display` impl arms

The `Display` trait for `Arch` has a manual `match` — new variants must be added explicitly:

```rust
Self::AmdgpuGfx11   => write!(f, "amdgpu-gfx11")?,
Self::AmdgpuGfx950  => write!(f, "amdgpu-gfx950")?,
Self::AmdgpuGfx12   => write!(f, "amdgpu-gfx12")?,
Self::AmdgpuGfx1250 => write!(f, "amdgpu-gfx1250")?,
```

> **Pitfall**: Forgetting to update `Display` causes a non-exhaustive pattern compile error at the `match self` in `fmt()`.

---

### Step 3: Adding the JSON Parser

**File**: `asm-lsp/parser.rs`

Added `populate_amdgpu_instructions` following the same pattern as the existing `populate_mips_instructions`. The AMD JSON format is identical to the MIPS format (a flat array of `{name, summary, asm_templates}` objects), so the parser is nearly identical:

```rust
pub fn populate_amdgpu_instructions(arch: Arch, json_contents: &str) -> Result<Vec<Instruction>> {
    #[derive(Deserialize, Debug)]
    struct AmdgpuInstruction {
        pub name: String,
        pub summary: String,
        pub asm_templates: Vec<String>,
    }

    let raw_instrs: Vec<AmdgpuInstruction> =
        serde_json::from_str(json_contents).map_err(|e| anyhow!("Failed to parse JSON: {e}"))?;
    let instructions: Vec<Instruction> = raw_instrs
        .into_iter()
        .map(|instr| Instruction {
            name: instr.name.to_ascii_lowercase(),
            summary: instr.summary,
            asm_templates: instr.asm_templates,
            arch,
            forms: Vec::new(),
            aliases: Vec::new(),
            url: None,
        })
        .collect();

    Ok(instructions)
}
```

The key difference from MIPS: the function takes `arch: Arch` as a parameter, allowing the same parser to be reused for all four AMD generations.

Registers use the existing `populate_registers()` generic XML parser — no new register parser was needed, since the AMD register XML was written to match the expected format.

---

### Step 4: Wiring the Doc Serializer

**File**: `asm_docs_parsing/src/main.rs`

This binary is called by `xtask` to convert source docs (JSON/XML/etc.) into bincode. Two changes:

#### 4a. Import the new parser

```rust
use asm_lsp::parser::populate_amdgpu_instructions;
```

#### 4b. Add match arms for all four AMD variants

Inside the `(false, arch_in)` branch of `run()`:

```rust
Some(arch @ Arch::AmdgpuGfx11)
| Some(arch @ Arch::AmdgpuGfx950)
| Some(arch @ Arch::AmdgpuGfx12)
| Some(arch @ Arch::AmdgpuGfx1250) => {
    instrs = populate_amdgpu_instructions(arch, &conts)?;
}
```

The `arch @` binding captures the matched variant so it can be passed to the parser (which needs to know which `Arch` to stamp on each instruction).

---

### Step 5: Wiring xtask

**File**: `xtask/src/regnerate_docs.rs`

The xtask `regenerate-docs` subcommand builds and runs `asm_docs_parsing` for each architecture. Added entries for AMD opcodes and registers.

#### Opcodes (added to `gen_opcodes()`)

```rust
for (gpu_gen, arch_arg) in [
    ("amdgpu-gfx11",   "amdgpu-gfx11"),
    ("amdgpu-gfx950",  "amdgpu-gfx950"),
    ("amdgpu-gfx12",   "amdgpu-gfx12"),
    ("amdgpu-gfx1250", "amdgpu-gfx1250"),
] {
    println!("\t{gpu_gen}");
    Command::new(PARSE_EXE)
        .args([
            &format!("docs_store/opcodes/{gpu_gen}.json"),
            "-o",
            &format!("asm-lsp/serialized/opcodes/{gpu_gen}"),
        ])
        .arg("--doc-type").arg("instruction")
        .arg("--arch").arg(arch_arg)
        .current_dir(root_path)
        .status()
        .with_context(|| anyhow!("Failed to regenerate serialized {gpu_gen} opcodes"))?;
}
```

> **Pitfall**: `gen` is a reserved keyword in Rust 2024 edition. Using it as a loop variable name causes a compile error. Renamed to `gpu_gen`.

#### Registers (added to `gen_registers()`)

```rust
println!("\tamdgpu");
Command::new(PARSE_EXE)
    .args([
        "docs_store/registers/amdgpu.xml",
        "-o",
        "asm-lsp/serialized/registers/amdgpu",
    ])
    .arg("--doc-type").arg("register")
    .arg("--arch").arg("amdgpu-gfx11")
    .current_dir(root_path)
    .status()
    .with_context(|| anyhow!("Failed to regenerate serialized amdgpu registers"))?;
```

Note: `--arch amdgpu-gfx11` is passed, but the arch embedded in the XML file's `InstructionSet name` attribute is what actually gets used by the parser.

---

### Step 6: Config Builder

**File**: `asm-lsp/config_builder.rs`

Extended `ARCH_LIST` from 11 to 15 entries:

```rust
const ARCH_LIST: [Arch; 15] = [
    Arch::X86,
    Arch::X86_64,
    Arch::X86_AND_X86_64,
    Arch::ARM,
    Arch::ARM64,
    Arch::RISCV,
    Arch::Z80,
    Arch::MOS6502,
    Arch::PowerISA,
    Arch::Avr,
    Arch::Mips,
    Arch::AmdgpuGfx11,   // new
    Arch::AmdgpuGfx950,  // new
    Arch::AmdgpuGfx12,   // new
    Arch::AmdgpuGfx1250, // new
];
```

This makes the four AMD variants appear in the interactive `asm-lsp gen-config` architecture selection menu.

---

### Step 7: JSON Schema

**File**: `asm-lsp_config_schema.json`

Added four values to the `Arch` enum in the schema (used by editors for `.asm-lsp.toml` validation and autocomplete):

```json
"enum": [
  "x86", "x86-64", "arm", "arm64", "riscv", "z80",
  "6502", "power-isa", "AVR", "mips",
  "amdgpu-gfx11",
  "amdgpu-gfx950",
  "amdgpu-gfx12",
  "amdgpu-gfx1250"
]
```

---

### Step 8: Running Serialization

Before running `xtask`, placeholder (empty) serialized files must exist so the `include_bytes!()` macro in `types.rs` can compile:

```bash
touch asm-lsp/serialized/opcodes/amdgpu-gfx11 \
      asm-lsp/serialized/opcodes/amdgpu-gfx950 \
      asm-lsp/serialized/opcodes/amdgpu-gfx12 \
      asm-lsp/serialized/opcodes/amdgpu-gfx1250 \
      asm-lsp/serialized/registers/amdgpu
```

Then build and regenerate:

```bash
cargo build --workspace
cargo xtask regenerate-docs
```

The xtask:
1. Builds the whole workspace in release mode
2. Runs `asm_docs_parsing` for each architecture to produce bincode files
3. The AMD JSON files are parsed by `populate_amdgpu_instructions` and serialized

Final serialized file sizes:
```
asm-lsp/serialized/opcodes/amdgpu-gfx11      247 KB
asm-lsp/serialized/opcodes/amdgpu-gfx950     222 KB
asm-lsp/serialized/opcodes/amdgpu-gfx12      240 KB
asm-lsp/serialized/opcodes/amdgpu-gfx1250    326 KB
asm-lsp/serialized/registers/amdgpu           62 KB
```

---

### Step 9: Tests

**File**: `asm-lsp/test.rs`

Added four serialization freshness tests — these verify that the bincode files exactly match what would be produced by re-parsing the source JSON, catching any drift between source and serialized data:

```rust
#[test]
fn serialized_amdgpu_gfx11_instructions_are_up_to_date() {
    serialized_instructions_test!(
        "serialized/opcodes/amdgpu-gfx11",
        "../docs_store/opcodes/amdgpu-gfx11.json",
        |s| populate_amdgpu_instructions(Arch::AmdgpuGfx11, s)
    );
}
// ... same pattern for gfx950, gfx12, gfx1250
```

The `serialized_instructions_test!` macro takes a populate function with signature `fn(&str) -> Result<Vec<Instruction>>`. Since `populate_amdgpu_instructions` takes two arguments `(Arch, &str)`, a closure `|s| populate_amdgpu_instructions(Arch::AmdgpuGfx11, s)` is used to adapt it.

Also updated the imports in the `tests` module:

```rust
use crate::parser::populate_amdgpu_instructions;
```

**Result**: 147 tests pass, 0 failed.

---

## Data Pipeline Details

### llvm-tblgen JSON Format

The `--dump-json` output is a JSON object where each key is a TableGen record name:

```json
{
  "!instanceof": { "Instruction": ["V_ADD_F32_e32", "V_ADD_F32_e32_gfx11", ...] },
  "V_ADD_F32_e32_gfx11": {
    "isPseudo": 0,
    "isCodeGenOnly": 0,
    "AsmString": "v_add_f32$vdst, $src0, $src1",
    "SubtargetPredicate": {"printable": "TruePredicate"},
    "!superclasses": ["VOP_Real", "Instruction", "VOP2", ...],
    ...
  }
}
```

Key fields:
- `!instanceof["Instruction"]` — list of all record names that are instances of the `Instruction` class
- `isPseudo` / `isCodeGenOnly` — filters for non-assembler-visible instructions
- `AsmString` — assembly syntax template (uses `$operand` placeholders)
- `SubtargetPredicate` — which subtarget features this instruction requires
- `!superclasses` — class hierarchy (VOP1, VOP2, MIMG, DS, FLAT, etc.)

### Filtering Real Instructions

A "real" (assembler-visible) instruction satisfies all three:
1. `isPseudo == 0` — not a compiler-internal pseudo-instruction
2. `isCodeGenOnly == 0` — not hidden from the assembler
3. Non-empty `AsmString` — has actual assembly syntax

The script then classifies by generation using the record name suffix:
- Records ending in `_gfx11` → GFX11
- Records ending in `_gfx12` → GFX12
- Records ending in `_gfx1250` → GFX1250
- Records with `SubtargetPredicate` containing `HasGFX950Insts` → GFX950-specific

There are ~8,000–8,400 real instruction records per generation. After deduplication by mnemonic (keeping all unique assembly templates per mnemonic), this reduces to ~1,400 unique mnemonics per generation.

### AsmString Cleaning

The raw `AsmString` from tblgen has a quirky format:

```
"v_add_f32$vdst, $src0, $src1"              # no space after mnemonic
"v_cmpx_lg_f64{_e64} $src0_modifiers, ..."  # optional suffix in braces
"s_mov_b32$sdst, $src0$clamp"               # optional modifier at end
```

The `clean_asm_string()` function in the script:

1. **Inserts space** between mnemonic and first operand:
   `v_add_f32$vdst` → `v_add_f32 $vdst`
   (regex: `([a-z0-9_])(\$)` → `\1 \2`)

2. **Removes optional modifiers** like `$clamp`, `$omod`, `$dpp_ctrl`, etc.:
   `$vdst, $src0, $src1$clamp` → `$vdst, $src0, $src1`

3. **Removes optional suffixes** in braces:
   `v_cmpx_lg_f64{_e64}` → `v_cmpx_lg_f64`

4. **Strips `$` from operand names**:
   `$vdst, $src0, $src1` → `vdst, src0, src1`

5. **Collapses whitespace** and removes trailing commas.

Then the entire string is uppercased for the `asm_templates` field, following the convention of other architectures in asm-lsp.

### GFX950 Special Handling

GFX950 (CDNA4, MI350) is a GFX9-family processor — it's not a new instruction encoding family like GFX11 or GFX12. Its unique instructions use the `HasGFX950Insts` predicate rather than a `_gfx950` suffix.

Strategy:
1. Start with all `_gfx9` suffix instructions (base GFX9 set)
2. Add all `_vi` suffix instructions (GFX8/Volcanic Islands, also present on GFX9)
3. Add all instructions with `HasGFX950Insts` in their `SubtargetPredicate` (133 unique new MFMA/matrix instructions)
4. Deduplicate by mnemonic with `merge_instrs()`

The GFX950-specific instructions are primarily new MFMA variants:
- `v_mfma_f32_16x16x32_f16` — FP16 matrix FMA
- `v_mfma_f32_32x32x64_f8f6f4` — FP8/BF8/FP6/BF6/FP4 mixed-precision matrix FMA
- `v_smfmac_*` — Sparse matrix FMA
- New `v_cvt_f32_bf16` BF16 conversion instructions

### GFX1250 Merging Strategy

GFX1250 (CDNA4) is a GFX12-family processor with additional CDNA4-specific compute instructions. The assembly syntax for GFX12 base instructions is identical on GFX1250.

Strategy:
1. Start with all GFX12 instructions (`_gfx12` suffix)
2. Add all GFX1250-specific instructions (`_gfx1250` suffix)
3. Add instructions with `isGFX1250Plus` or `isGFX125xOnly` predicates
4. Deduplicate by mnemonic

Result: GFX1250 gets all 1,429 GFX12 instructions plus ~215 new ones = 1,644 total.

The GFX1250-specific additions include:
- `cluster_load_async_to_lds_*` — new async cluster load instructions
- Various new FP8/FP6/FP4 conversion and matrix instructions

### Register XML Format

The `populate_registers()` XML parser expects a specific format. Key requirements discovered through debugging:

1. **Root element**: `<InstructionSet name="...">` — the `name` attribute must be a valid `Arch` strum serialization string (e.g. `"amdgpu-gfx11"`)

2. **Explicit close tags**: `<Register>...</Register>` — the parser only triggers on `Event::End(QName(b"Register"))`, which does NOT fire for self-closing `<Register />` tags (those fire `Event::Empty` instead). This was the primary bug causing "Zero registers read in."

3. **`type` attribute**: Must match `RegisterType` strum serialization, e.g. `"General Purpose Register"`, `"Special Purpose Register"`

4. **`width` attribute**: Must match `RegisterWidth` strum serialization, e.g. `"32 bits"`, `"64 bits"`

The register set includes:
- **VGPR v0–v255**: Vector General Purpose Registers (32-bit per lane, up to 64 lanes on wave64)
- **AGPR a0–a255**: Accumulation VGPRs (CDNA-specific, used for MFMA instructions)
- **SGPR s0–s105**: Scalar General Purpose Registers (32-bit, uniform across wavefront)
- **TTMP ttmp0–ttmp15**: Trap Temporary Registers (privileged, exception handling)
- **Special**: `exec`, `exec_lo/hi`, `vcc`, `vcc_lo/hi`, `m0`, `scc`, `flat_scratch`, `xnack_mask`, etc.

---

## Files Changed

### Modified files

| File | Change summary |
|---|---|
| `asm-lsp/types.rs` | +4 `Arch` variants, `setup_registers/instructions` match arms, `Display` impl arms |
| `asm-lsp/parser.rs` | Added `populate_amdgpu_instructions(arch, json_str)` |
| `asm_docs_parsing/src/main.rs` | Import + 4 match arms for AMD variants |
| `xtask/src/regnerate_docs.rs` | AMD opcode loop + AMD register entry in `gen_registers()` |
| `asm-lsp/config_builder.rs` | `ARCH_LIST` size 11 → 15, +4 AMD variants |
| `asm-lsp_config_schema.json` | +4 enum values to `instruction_set` |
| `asm-lsp/test.rs` | Import + 4 serialization freshness tests |

### New files

| File | Purpose |
|---|---|
| `scripts/gen_amdgpu_docs.py` | Python script: tblgen JSON → asm-lsp JSON/XML |
| `docs_store/opcodes/amdgpu-gfx11.json` | GFX11 instruction data (1,373 mnemonics) |
| `docs_store/opcodes/amdgpu-gfx950.json` | GFX950 instruction data (1,478 mnemonics) |
| `docs_store/opcodes/amdgpu-gfx12.json` | GFX12 instruction data (1,429 mnemonics) |
| `docs_store/opcodes/amdgpu-gfx1250.json` | GFX1250 instruction data (1,644 mnemonics) |
| `docs_store/registers/amdgpu.xml` | Shared register definitions (634 registers) |
| `asm-lsp/serialized/opcodes/amdgpu-gfx11` | Bincode (generated) |
| `asm-lsp/serialized/opcodes/amdgpu-gfx950` | Bincode (generated) |
| `asm-lsp/serialized/opcodes/amdgpu-gfx12` | Bincode (generated) |
| `asm-lsp/serialized/opcodes/amdgpu-gfx1250` | Bincode (generated) |
| `asm-lsp/serialized/registers/amdgpu` | Bincode (generated) |
| `AMD_ISA_SUPPORT.md` | This document |

---

## Usage

### Quick start

1. Install or build `asm-lsp` from this repo:
   ```bash
   cargo install --path asm-lsp
   ```

2. Create `.asm-lsp.toml` in your project root:
   ```toml
   [default_config]
   assembler = "gas"
   instruction_set = "amdgpu-gfx12"
   ```

3. Configure your editor to use `asm-lsp` as the LSP server for `.s` / `.asm` files.

### Available instruction sets

| Value | Hardware |
|---|---|
| `amdgpu-gfx11` | Radeon RX 7000 series, Instinct MI200/MI300 |
| `amdgpu-gfx950` | Instinct MI350 (CDNA4 (ISA 9.5.0)) |
| `amdgpu-gfx12` | Radeon RX 9000 series (RDNA4) |
| `amdgpu-gfx1250` | Instinct MI400+ (CDNA4) |

### Interactive config generation

```bash
asm-lsp gen-config
```

The four AMD variants will appear in the architecture selection menu.

### Regenerating data from scratch

If you want to update the instruction data (e.g. after a new LLVM version):

```bash
# 1. Dump TableGen JSON (requires matching llvm-tblgen binary)
TBLGEN=<path-to-llvm-tblgen>
LLVM_AMDGPU=<path-to-llvm>/lib/Target/AMDGPU
LLVM_INC=<path-to-llvm>/include

$TBLGEN --dump-json -I $LLVM_INC -I $LLVM_AMDGPU $LLVM_AMDGPU/AMDGPU.td \
  -o /tmp/amdgpu_all.json

# 2. Extract instruction + register data
python3 scripts/gen_amdgpu_docs.py /tmp/amdgpu_all.json .

# 3. Serialize to bincode
cargo xtask regenerate-docs

# 4. Run tests to verify
cargo test
```

---

## Instruction Counts

After deduplication by mnemonic:

| Generation | Unique mnemonics | Notes |
|---|---|---|
| GFX11 | 1,373 | RDNA3/CDNA2-3 real instructions |
| GFX950 | 1,478 | GFX9 base + GFX9-vi + 133 GFX950-specific |
| GFX12 | 1,429 | RDNA4 real instructions |
| GFX1250 | 1,644 | GFX12 base + ~215 CDNA4 additions |
| **Registers** | **634** | Shared across all AMD generations |

---

## Known Limitations and Future Work

1. **No URL links**: Instruction hover text does not include links to AMD ISA documentation PDFs. These could be added by mapping mnemonics to sections in the publicly available AMD ISA reference guides.

2. **Simplified descriptions**: The `summary` field is auto-generated from the instruction class and generation label, not from actual prose descriptions. The AMD ISA PDFs contain detailed per-instruction descriptions but are not machine-readable.

3. **Operand type information**: The `forms` field of `Instruction` is left empty. Filling it with proper operand type information would enable signature help (showing parameter types as you type). This would require parsing the TableGen operand classes more deeply.

4. **No assembler directives**: AMD GPU assembly commonly uses GAS-compatible directives (`.type`, `.globl`, `.amdgcn_target`, etc.) which are not AMD-specific and are already covered by the existing GAS directive support.

5. **Wave32 vs Wave64**: Some instructions behave differently on wave32 (RDNA) vs wave64 (CDNA/GCN) but the current implementation does not distinguish this within a generation.

6. **GFX9 base support**: GFX950 includes GFX9/VI-era instructions, but there is no explicit `amdgpu-gfx9` config option for older Vega hardware (gfx900/gfx906). This could be added following the same pattern.

---

## Gotchas and Debugging Notes

These issues were encountered during implementation and may be useful if you hit them:

### 1. `gen` is a reserved keyword (Rust 2024)

Using `gen` as a loop variable name in `xtask/src/regnerate_docs.rs` causes:
```
error: expected identifier, found reserved keyword `gen`
```
Workaround: rename to `gpu_gen` or any non-reserved name.

### 2. Self-closing XML tags → "Zero registers read in"

The `populate_registers()` XML parser in `parser.rs` only adds a register to the map when it sees `Event::End(QName(b"Register"))`. Self-closing tags (`<Register .../>`) fire `Event::Empty`, which the parser ignores.

**Wrong**: `<Register name="v0" .../>`
**Correct**: `<Register name="v0" ...>\n  </Register>`

### 3. AsmString has no space between mnemonic and first operand

LLVM TableGen `AsmString` values look like `"v_add_f32$vdst, $src0, $src1"` — the mnemonic is immediately followed by `$vdst` without a space. Naively stripping `$` gives `"v_add_f32vdst, ..."` where the mnemonic extraction reads `v_add_f32vdst` as the mnemonic.

Fix: before stripping `$`, insert a space between a lowercase word character and `$`:
```python
asm = re.sub(r'([a-z0-9_])(\$)', r'\1 \2', asm)
```

### 4. tblgen binary version must match source

Running `llvm-tblgen` version 16 against LLVM 22 source files produces parse errors:
```
error: Variable not defined: 'al'
```
Always use the `llvm-tblgen` binary built from the same LLVM checkout as the `.td` files.

### 5. InstructionSet name in register XML must be a valid Arch string

`populate_registers()` does `Arch::from_str(name_attr)` on the `InstructionSet` element's `name` attribute. If `name="amdgpu"` (not a valid strum string), it panics. Use one of the registered strum values: `"amdgpu-gfx11"`.

### 6. Placeholder serialized files needed before first compile

`include_bytes!("serialized/opcodes/amdgpu-gfx11")` in `types.rs` causes a compile error if the file does not exist. Create empty placeholder files with `touch` before running `cargo build`, then replace them with real content via `cargo xtask regenerate-docs`.

---

## Phase 2: Rich Instruction Hover via ISA PDF Annotations

After the initial implementation, instruction hover text showed only a terse auto-generated summary such as:

```
Scalar ALU (2 sources). Available on GFX11 (RDNA3/CDNA2-3).
```

This phase added per-instruction descriptions, operation pseudocode, and notes extracted directly from AMD's public ISA reference PDFs.

### Plan

1. Write `scripts/parse_amd_isa_pdf.py` — extract annotations from ISA PDFs using `pdftotext`
2. Update `scripts/gen_amdgpu_docs.py` — load annotations, embed them as markdown in `summary`
3. Generate annotation JSON files from all 4 ISA PDFs
4. Regenerate opcode JSON and serialized bincode
5. Verify hover output in Neovim; run `cargo test`

### Target hover output format

```
s_add_i32 [amdgpu-gfx11]

Add two signed inputs, store the result into a scalar register and store the carry-out bit into SCC.

**Operation:**
```
tmp = S0.i + S1.i;
SCC = ((S0.u[31] == S1.u[31]) && (S0.u[31] != tmp.u[31]));
// signed overflow.
D0.i = tmp.i
```

**Notes:**
This opcode is not suitable for use with S_ADDC_U32 for implementing 64-bit operations.

## Templates
 + `S_ADD_I32 SDST, SSRC0, SSRC1`
```

### Step 1: `scripts/parse_amd_isa_pdf.py`

A new script that converts AMD ISA PDFs to per-instruction annotation JSON using `pdftotext -layout -nopgbrk`.

#### PDF text format

`pdftotext -layout` on AMD ISA PDFs yields a consistent structure per instruction:

```
INSTRUCTION_NAME                                                      N

Description prose text.


  pseudocode_line1;
  pseudocode_line2;


Notes

Notes text here.


NEXT_INSTRUCTION_NAME                                                N+1
```

Verified to be consistent across RDNA3, RDNA4, MI300 (CDNA3), and CDNA4 PDFs.

#### Instruction boundary detection

```python
INSTR_NAME_RE = re.compile(r'^([A-Z][A-Z0-9_]{3,})\s*\d*\s*$')

_SECTION_HEADERS = {
    'SOP1', 'SOP2', 'SOPC', 'SOPK', 'SOPP',
    'VOP1', 'VOP2', 'VOP3', 'VOPC', 'VOP3P', 'VOPD',
    'ENCODING', 'DESCRIPTION', 'OPERATION', ...
}
_PREFIXES = (
    'S_', 'V_', 'DS_', 'FLAT_', 'GLOBAL_', 'SCRATCH_',
    'BUFFER_', 'IMAGE_', 'TBUFFER_', 'VINTRP_', 'EXP', ...
)

def is_instruction_name(name):
    return name not in _SECTION_HEADERS and any(name.startswith(p) for p in _PREFIXES)
```

#### Noise filtering

Page footers are discarded:
- Pure number lines (page numbers): `^\d+\s*$`
- Section footer lines: `^\d+\.\d+` (e.g. `"16.1. SOP2 Instructions   188 of 600"`)
- ISA title lines: `"RDNA3" Instruction Set Architecture`

#### State-machine body parser

Three states: `description` → `operation` → `notes`

- **description**: non-empty, non-indented lines before any pseudocode
- **operation**: lines with 2+ leading spaces (indented), or non-indented lines matching code heuristics (`=`, `;`, `if (`, `SCC`, `VCC`, etc.)
- **notes**: all lines after a bare `Notes` or `Note:` line

#### Output

```json
{
  "s_add_i32": {
    "description": "Add two signed inputs, store the result...",
    "operation": "tmp = S0.i + S1.i;\nSCC = ...;\nD0.i = tmp.i",
    "notes": "This opcode is not suitable for use with S_ADDC_U32..."
  }
}
```

When the same mnemonic appears multiple times (e.g. once in a summary table, once in the detail section), the entry with more content wins.

#### Source PDFs

| PDF file | Covers | Annotations |
|---|---|---|
| `rdna3-shader-instruction-set-architecture-feb-2023_0.pdf` | GFX11 | 1,137 instructions |
| `amd-instinct-mi300-cdna3-instruction-set-architecture.pdf` | GFX950 | 1,144 instructions |
| `rdna4-instruction-set-architecture.pdf` | GFX12 | 1,260 instructions |
| `amd-instinct-cdna4-instruction-set-architecture.pdf` | GFX1250 | 1,232 instructions |

Run:
```bash
for gen in gfx11 gfx950 gfx12 gfx1250; do
  python3 scripts/parse_amd_isa_pdf.py \
    amd_isas/<matching_pdf> \
    -o docs_store/annotations/amdgpu-${gen}-annotations.json
done
```

### Step 2: Updating `scripts/gen_amdgpu_docs.py`

Two helpers added:

```python
def load_annotations(path: str) -> dict:
    """Load instruction annotations JSON; return {} if file absent."""
    try:
        with open(path) as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return {}

def build_summary(brief: str, annotation: dict | None) -> str:
    """Build markdown summary from annotation; fall back to brief if absent."""
    if not annotation:
        return brief
    desc = (annotation.get('description') or '').strip()
    op   = (annotation.get('operation')   or '').strip()
    notes = (annotation.get('notes')      or '').strip()
    if len(desc) < 10:   # too short to be meaningful
        return brief
    parts = [desc]
    if op:
        parts.append('**Operation:**\n```\n' + op + '\n```')
    if notes:
        parts.append('**Notes:**\n' + notes)
    return '\n\n'.join(parts)
```

Both `extract_by_suffix()` and `extract_by_predicate()` were updated to accept an `annotations: dict` parameter and call `build_summary(brief, annotations.get(mnemonic))`.

In `main()`, annotation files are loaded at startup:
```python
ann_gfx11   = load_annotations('docs_store/annotations/amdgpu-gfx11-annotations.json')
ann_gfx950  = load_annotations('docs_store/annotations/amdgpu-gfx950-annotations.json')
ann_gfx12   = load_annotations('docs_store/annotations/amdgpu-gfx12-annotations.json')
ann_gfx1250 = load_annotations('docs_store/annotations/amdgpu-gfx1250-annotations.json')
```

Instructions without PDF coverage retain the brief auto-generated summary unchanged.

### Step 3: Regenerating data

```bash
python3 scripts/gen_amdgpu_docs.py /tmp/amdgpu_all.json .
cargo xtask regenerate-docs
cargo test   # all 147 tests pass
cargo build --release
```

### New files added in Phase 2

| File | Purpose |
|---|---|
| `scripts/parse_amd_isa_pdf.py` | PDF → annotation JSON extractor |
| `docs_store/annotations/amdgpu-gfx11-annotations.json` | GFX11 annotations (1,137 instructions) |
| `docs_store/annotations/amdgpu-gfx950-annotations.json` | GFX950 annotations (1,144 instructions) |
| `docs_store/annotations/amdgpu-gfx12-annotations.json` | GFX12 annotations (1,260 instructions) |
| `docs_store/annotations/amdgpu-gfx1250-annotations.json` | GFX1250 annotations (1,232 instructions) |

### Coverage

PDF extraction covers approximately 80–90% of instructions by mnemonic (instructions unique to summary tables or with very short entries may be missed). Remaining instructions fall back to the brief auto-generated summary.

---

## Phase 3: Neovim Integration

This phase configured Neovim to automatically use the correct AMD ISA for assembly files and set up ROCm's assembler for accurate diagnostics.

### Plugin configuration (`~/.config/nvim/lua/plugins/asm-lsp.lua`)

Uses lazy.nvim to load the fork of asm-lsp:

```lua
{
    "bergercookie/asm-lsp",
    url = "https://github.com/longknown-amd/asm-lsp.git",
    branch = "feature/amdgpu-isa-support",
    build = "cargo build --release",
    ft = { "asm", "s", "S" },
    ...
}
```

### Auto-detection of GPU target

AMD GPU assembly files generated by LLVM contain a `.amdgcn_target` directive on one of the first few lines:

```
.amdgcn_target "amdgcn-amd-amdhsa--gfx950"
```

The Neovim config detects this and maps the GFX codename to an `instruction_set` value (verified against LLVM's `GCNProcessors.td`):

| GFX codename | `instruction_set` |
|---|---|
| `gfx1250`, `gfx1251` | `amdgpu-gfx1250` |
| `gfx950` | `amdgpu-gfx950` |
| `gfx1200`, `gfx1201`, `gfx12-generic` | `amdgpu-gfx12` |
| `gfx1100`–`gfx1103`, `gfx1150`–`gfx1153`, `gfx11-generic` | `amdgpu-gfx11` |

### Auto-creation of `.asm-lsp.toml`

Detection and TOML writing happens inside the `root_dir` callback of the LSP config — this runs synchronously before the server process starts, so the server sees the correct `instruction_set` on its first read.

```lua
root_dir = function(bufnr, on_dir)
    local gfx = detect_amdgcn_target(bufnr)
    if gfx then
        local isa = gfx_to_instruction_set(gfx)
        if isa and vim.fn.filereadable(toml_path) == 0 then
            vim.fn.writefile({
                '[default_config]',
                'assembler = "gas"',
                string.format('instruction_set = "%s"', isa),
                '',
                '[default_config.opts]',
                'compiler = "/opt/rocm/llvm/bin/llvm-mc"',
                string.format('compile_flags_txt = ["--triple=amdgcn-amd-amdhsa", "--mcpu=%s", "--filetype=asm"]', gfx),
            }, toml_path)
        end
    end
    on_dir(root)
end
```

If an existing `.asm-lsp.toml` has a mismatched `instruction_set`, a warning notification is shown instead.

### ROCm `llvm-mc` for diagnostics

The system `gcc`/`gas` assembler has no AMDGPU backend, so it reports spurious errors for every AMDGPU instruction. The auto-generated TOML uses `/opt/rocm/llvm/bin/llvm-mc` instead:

```toml
[default_config.opts]
compiler = "/opt/rocm/llvm/bin/llvm-mc"
compile_flags_txt = ["--triple=amdgcn-amd-amdhsa", "--mcpu=gfx950", "--filetype=asm"]
```

ROCm's `llvm-mc` handles all AMDGPU instructions, `.set` symbolic register names, and `//` line comments, and outputs standard `file:line:col: error:` diagnostics that asm-lsp parses correctly.

### Key gotcha: `root_dir` callback signature (Neovim 0.11)

Neovim 0.11's native LSP API changed the `root_dir` callback signature from `(fname: string)` to `(bufnr: integer, on_dir: function)`. Using the old signature causes:

```
Error: vim/fs.lua:0: path: expected string, got number
```

Fix: use `vim.api.nvim_buf_get_name(bufnr)` to convert the buffer number to a file path.
