# Language Server for NASM/GAS/GO Assembly

[![crates.io](https://img.shields.io/crates/v/asm-lsp.svg)](https://crates.io/crates/asm-lsp)
[![Tests](https://github.com/bergercookie/asm-lsp/actions/workflows/lint_and_test.yml/badge.svg)](https://github.com/bergercookie/asm-lsp/actions/workflows/lint_and_test.yml)

## Goal

Provide hovering, autocompletion, signature help, go to definition, and view
references for assembly files written in the GAS/NASM or GO assembly flavors. It
supports assembly files for the x86, x86_64, ARM, RISCV, and z80 instruction sets.

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
cargo install --git https://github.com/bergercookie/asm-lsp asm-lsp
```

Install using the cargo from source:

```bash
cargo install --path asm-lsp
```

### Precompiled Binaries

Download and uncompress the appropriate precompiled binaries from the project's
[releases page](https://github.com/bergercookie/asm-lsp/releases).

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
selectively target specific assemblers and/or instruction sets. By default, diagnostics
are enabled and the server attempts to invoke `gcc` (and then `clang`) to generate
them. If the `compiler` config field is specified, the server will attempt to use
the specified compiler to generate diagnostics. Different configurations can be
created for different sub-directories or files within your project as `project`s.
Source files not contained within any `project` configs will use the default configuration
if provided.

#### Config Builder (MASTER BRANCH ONLY)

Creating a `.asm-lsp.toml` file manually is fine, but can be error-prone as projects
grow in complexity. Running `asm-lsp gen-config` will walk you through the creation
of a config interactively, with informative prompts and extra validation checks
along the way.

```
$ asm-lsp gen-config --help
Generate a .asm-lsp.toml config file

Usage: asm-lsp gen-config [OPTIONS]

Options:
  -o, --output-dir <OUTPUT_DIR>      Directory to place .asm-lsp.toml into. (Default is the current directory)
  -g, --global-cfg                   Place the config in the global config directory
  -p, --project-path <PROJECT_PATH>  Path to the project this config is being generated for. (Default is the current directory)
  -w, --overwrite                    Overwrite any existing .asm-lsp.toml in the target directory
  -q, --quiet                        Don't display the generated config file after generation
  -h, --help                         Print help
```

#### NOTE

If the server reads in an invalid configuration file, it will display an error
message and exit.

```toml
[default_config]
# Configure documentation available for features like hover and completions
assembler = "go"
instruction_set = "x86/x86-64"

[opts]
# The `compiler` field is the name of a compiler/assembler on your path
# (or the absolute path to the file) that is used to build your source files
# This program will be used to generate diagnostics
compiler = "zig" # need "cc" as the first argument in `compile_flags.txt`
diagnostics = true
default_diagnostics = true

# Configure the server's treatment of source files in the `arm-project` sub-directory
[[project]]
path = "arm-project"
assembler = "gas"
instruction_set = "arm"

[project.opts]
compiler = "zig"
compile_flags_txt = [
  "cc",
  "-x",
  "assembler-with-cpp",
  "-g",
  "-Wall",
  "-Wextra",
  "-pedantic",
  "-pedantic-errors",
  "-std=c2y",
  "-target",
  "aarch64-linux-musl",
]
```

Valid options for the `instruction_set` field include:

- `"x86"`
- `"x86-64"`
- `"x86/x86-64"` (Enable both)
- `"arm"`
- `"arm64"`
- `"riscv"`
- `"z80"`

Valid options for the `assembler` field include:

- `"gas"`
- `"go"`
- `"masm"`
- `"nasm"`

Don't see an architecture and/or assembler that you'd like to work with? File an
[issue](https://github.com/bergercookie/asm-lsp/issues/new/choose)! We would be
happy to add new options to the tool.

### [OPTIONAL] Extend functionality via `compile_commands.json`/`compile_flags.txt`

Add a [`compile_commands.json`](https://clang.llvm.org/docs/JSONCompilationDatabase.html#format)
or [`compile_flags.txt`](https://clang.llvm.org/docs/JSONCompilationDatabase.html#alternatives)
file to your project's root or root `build` directory to enable inline diagnostic
features, as well as to specify additional include directories for use in hover
features. If a `compile_commands.json` or `compile_flags.txt` file isn't provided,
the server will attempt to provide diagnostics with a default compile command.
This feature can be disabled by setting the `default_diagnostics` config field
to `false`.

### VSCode Support

The project has not published any VSCode extension package yet. However, there is
a development extension in the [`editors/code`](https://github.com/bergercookie/asm-lsp/blob/master/editors/code/README.md)
directory with setup instructions.

## Root directory must contain `.git`

The lsp searches for a `.git` directory to locate the root of your project.
Please be sure to run `git init` if your project is not already configured as a
git repository.

## Demos / Features Documentation

### Hovering / Documentation support

![](https://github.com/bergercookie/asm-lsp/blob/master/demo/hover.gif)

### Autocomplete

![](https://github.com/bergercookie/asm-lsp/blob/master/demo/autocomplete.gif)

### Diagnostics

![](https://github.com/bergercookie/asm-lsp/blob/master/demo/diagnostics.gif)

### Goto Definition

![](https://github.com/bergercookie/asm-lsp/blob/master/demo/gotodef.gif)

### View References

![](https://github.com/bergercookie/asm-lsp/blob/master/demo/references.gif)

### Signature Help

![](https://github.com/bergercookie/asm-lsp/blob/master/demo/signaturehelp.gif)

- Triggering signature help is dependent on your editor and LSP client.
  - Using Neovim's built in LSP client, this can be done via the command
    `:lua vim.lsp.buf.signature_help()`.
  - Using coc, [this issue comment](https://github.com/neoclide/coc.nvim/issues/2656#issuecomment-845903417)
    suggests the remap `inoremap <silent> ,s <C-r>=CocActionAsync('showSignatureHelp')<CR>`
    to trigger signature help in insert mode.

## Acknowledgements / Sources

- x86 and x86-64 instruction documentation builds on top of the [opcodes python package](https://github.com/Maratyszcza/Opcodes)

- x86 and x86-64 register documentation sourced from:
  - OS Dev's [x86](https://wiki.osdev.org/CPU_Registers_x86) and [x86-64](https://wiki.osdev.org/CPU_Registers_x86-64)
    pages
  - Various [Wikipedia pages](https://en.wikipedia.org/wiki/Streaming_SIMD_Extensions)
    for SIMD specifics

- GAS directives sourced from SourceWare's [pseudo-ops page](https://sourceware.org/binutils/docs-2.41/as/Pseudo-Ops.html)

- MASM and NASM directives sourced from the [asm-dude](https://github.com/HJLebbink/asm-dude)
    Visual Studio extension project. Additions sourced from [Microsoft](https://learn.microsoft.com/en-us/cpp/assembler/masm/directives-reference?view=msvc-170)
    and [NASM](https://www.nasm.us/xdoc/2.13.03/html/nasmdoc0.html) documentation.

- ARM instruction documentation builds on top of ARM's official [Exploration tools documentation](https://developer.arm.com/Architectures/A-Profile%20Architecture#Downloads)

- RISCV instruction and register documentation builds on top of the [riscv-isadoc project](https://github.com/msyksphinz-self/riscv-isadoc?tab=CC-BY-4.0-1-ov-file)
