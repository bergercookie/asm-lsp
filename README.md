# Language Server for GAS/GO Assembly

[![crates.io](https://img.shields.io/crates/v/asm-lsp.svg)](https://crates.io/crates/asm-lsp)
[![Tests](https://github.com/bergercookie/asm-lsp/actions/workflows/lint_and_test.yml/badge.svg)](https://github.com/bergercookie/asm-lsp/actions/workflows/lint_and_test.yml)

## Goal

Provide hovering, autocompletion, and signature help for assembly files written
in the GAS/NASM or GO assembly flavors. It supports assembly files for the x86
or x86_64 instruction sets.

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
and/or `~/.config/asm-lsp/` (project configs will override global configs) to
selectively target specific assemblers and/or instruction sets.

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

![](https://github.com/bergercookie/asm-lsp/blob/master/demo/autocomplete.gif)

### Signature Help

![](https://github.com/bergercookie/asm-lsp/blob/master/demo/signaturehelp.gif)

- Triggering signature help is dependent on your editor and LSP client.
  - Using Neovim's built in LSP client, this can be done via the command
    ``:lua vim.lsp.buf.signature_help()``.
  - Using coc, [this issue comment](https://github.com/neoclide/coc.nvim/issues/2656#issuecomment-845903417)
    suggests the remap ``inoremap <silent> ,s <C-r>=CocActionAsync('showSignatureHelp')<CR>``
    to trigger signature help in insert mode.

## Acknowledgements

Current rust package builds on top of the [opcodes python
package](https://github.com/Maratyszcza/Opcodes)
