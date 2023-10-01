# Language Server for GAS/GO Assembly

[![crates.io](https://img.shields.io/crates/v/asm-lsp.svg)](https://crates.io/crates/asm-lsp)
[![Tests](https://github.com/bergercookie/asm-lsp/actions/workflows/lint_and_test.yml/badge.svg)](https://github.com/bergercookie/asm-lsp/actions/workflows/lint_and_test.yml)

## Goal

Provide hovering and (TODO) autocompletion for assembly files written in the
GAS/NASM or GO assembly flavors. It supports assembly files for the x86 or
x86_64 instruction sets.

This tool can serve as reference when reading the assembly output of a program.
This way you can query what each command exactly does and deliberate about
whether the compiler is producing the desired output or whether you have to
tweak your code for optimisation.

## Installation

### Using cargo

Install using the cargo package manager, either from `crates.io` or from github:

```bash
cargo install asm-lsp
# or to get the latest version from github
cargo install --git https://github.com/bergercookie/asm-lsp
```

## Set up as a language server

Add a section like the following in your `settings.json` file:

```json
"asm-lsp": {
    "command": "asm-lsp",
    "filetypes": [
        "asm", "s", "S"
    ]
}
```

### [OPTIONAL] Configure via `.asm-lsp.toml`

Add a `.asm-lsp.toml` file like the following to your project's root directory
to selectively target specific assemblers and/or instruction sets:

```toml
version = "0.1"

[assemblers]
gas = true
go = false

[instruction_sets]
x86 = false
x86_64 = true
```

## Demo

### Hovering / Documentation support

![](https://github.com/bergercookie/asm-lsp/blob/master/demo/hover.gif)

### Autocomplete

TODO

## Acknowledgements

Current rust package builds on top of the [opcodes python
package](https://github.com/Maratyszcza/Opcodes)
