# Language Server for NASM/GAS/GO Assembly

[![crates.io](https://img.shields.io/crates/v/asm-lsp.svg)](https://crates.io/crates/asm-lsp)
[![Tests](https://github.com/bergercookie/asm-lsp/actions/workflows/lint_and_test.yml/badge.svg)](https://github.com/bergercookie/asm-lsp/actions/workflows/lint_and_test.yml)

## Goal

Provide hovering, autocompletion, signature help, go to definition, and view
references for assembly files written in the GAS/NASM or GO assembly flavors. It
supports assembly files for the x86, x86_64, ARM, RISCV, z80, AVR, and 6502 instruction
sets. Supported assemblers include the Gas, Go, Masm, Nasm, ca65, AVR, and FASM
assemblers.

This tool can serve as reference when reading the assembly output of a program.
This way you can query what each command exactly does and deliberate about
whether the compiler is producing the desired output or whether you have to
tweak your code for optimisation.

## Installation

[![MSRV](https://img.shields.io/badge/MSRV-1.82.0-blue)](https://www.rust-lang.org/)

**Note**: The minimum supported Rust version (MSRV) is **1.88.0**, but this may change on the `master` branch.
...

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

#### Config Builder

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
- `"6502"`
- `"avr"`
- `"mips"`

Valid options for the `assembler` field include:

- `"gas"`
- `"go"`
- `"masm"`
- `"nasm"`
- `"ca65"`
- `"avr"`
- `"fasm"`
- `"mars"`

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
    and other [NASM](https://www.nasm.us/xdoc/2.13.03/html/nasmdoc0.html) [documentation](https://www.nasm.us/xdoc/2.16.03/html/nasmdoc7.html).

- ARM instruction documentation builds on top of ARM's official [Exploration tools documentation](https://developer.arm.com/Architectures/A-Profile%20Architecture#Downloads)

- RISCV instruction and register documentation builds on top of the [riscv-isadoc project](https://github.com/msyksphinz-self/riscv-isadoc?tab=CC-BY-4.0-1-ov-file)

- 6502 instruction and register documentation sourced from [masswerk](https://www.masswerk.at/6502/6502_instruction_set.html)

- CA65 assembler documentation sourced from the [cc65](https://github.com/cc65/doc)
    project's [documentation](https://cc65.github.io/doc/ca65.html)

- PowerISA instruction documentation sourced from the [open-power-sdk](https://github.com/open-power-sdk)
    PowerISA [documentation](https://github.com/open-power-sdk/PowerISA?tab=Apache-2.0-1-ov-file)*

- AVR instruction documentation sourced from the [AVR documentation](https://ww1.microchip.com/downloads/en/DeviceDoc/AVR-InstructionSet-Manual-DS40002198.pdf)

- AVR assembler documentation sourced from the [AVR documentation](https://ww1.microchip.com/downloads/en/DeviceDoc/40001917A.pdf)

- AVR register documentation sourced from the [AVR documentation](https://developerhelp.microchip.com/xwiki/bin/view/products/mcu-mpu/8-bit-avr/structure/gpr/)

<details><summary>* Licensed under Apache 2.0</summary><p>

```
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

   1. Definitions.

      "License" shall mean the terms and conditions for use, reproduction,
      and distribution as defined by Sections 1 through 9 of this document.

      "Licensor" shall mean the copyright owner or entity authorized by
      the copyright owner that is granting the License.

      "Legal Entity" shall mean the union of the acting entity and all
      other entities that control, are controlled by, or are under common
      control with that entity. For the purposes of this definition,
      "control" means (i) the power, direct or indirect, to cause the
      direction or management of such entity, whether by contract or
      otherwise, or (ii) ownership of fifty percent (50%) or more of the
      outstanding shares, or (iii) beneficial ownership of such entity.

      "You" (or "Your") shall mean an individual or Legal Entity
      exercising permissions granted by this License.

      "Source" form shall mean the preferred form for making modifications,
      including but not limited to software source code, documentation
      source, and configuration files.

      "Object" form shall mean any form resulting from mechanical
      transformation or translation of a Source form, including but
      not limited to compiled object code, generated documentation,
      and conversions to other media types.

      "Work" shall mean the work of authorship, whether in Source or
      Object form, made available under the License, as indicated by a
      copyright notice that is included in or attached to the work
      (an example is provided in the Appendix below).

      "Derivative Works" shall mean any work, whether in Source or Object
      form, that is based on (or derived from) the Work and for which the
      editorial revisions, annotations, elaborations, or other modifications
      represent, as a whole, an original work of authorship. For the purposes
      of this License, Derivative Works shall not include works that remain
      separable from, or merely link (or bind by name) to the interfaces of,
      the Work and Derivative Works thereof.

      "Contribution" shall mean any work of authorship, including
      the original version of the Work and any modifications or additions
      to that Work or Derivative Works thereof, that is intentionally
      submitted to Licensor for inclusion in the Work by the copyright owner
      or by an individual or Legal Entity authorized to submit on behalf of
      the copyright owner. For the purposes of this definition, "submitted"
      means any form of electronic, verbal, or written communication sent
      to the Licensor or its representatives, including but not limited to
      communication on electronic mailing lists, source code control systems,
      and issue tracking systems that are managed by, or on behalf of, the
      Licensor for the purpose of discussing and improving the Work, but
      excluding communication that is conspicuously marked or otherwise
      designated in writing by the copyright owner as "Not a Contribution."

      "Contributor" shall mean Licensor and any individual or Legal Entity
      on behalf of whom a Contribution has been received by Licensor and
      subsequently incorporated within the Work.

   2. Grant of Copyright License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      copyright license to reproduce, prepare Derivative Works of,
      publicly display, publicly perform, sublicense, and distribute the
      Work and such Derivative Works in Source or Object form.

   3. Grant of Patent License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      (except as stated in this section) patent license to make, have made,
      use, offer to sell, sell, import, and otherwise transfer the Work,
      where such license applies only to those patent claims licensable
      by such Contributor that are necessarily infringed by their
      Contribution(s) alone or by combination of their Contribution(s)
      with the Work to which such Contribution(s) was submitted. If You
      institute patent litigation against any entity (including a
      cross-claim or counterclaim in a lawsuit) alleging that the Work
      or a Contribution incorporated within the Work constitutes direct
      or contributory patent infringement, then any patent licenses
      granted to You under this License for that Work shall terminate
      as of the date such litigation is filed.

   4. Redistribution. You may reproduce and distribute copies of the
      Work or Derivative Works thereof in any medium, with or without
      modifications, and in Source or Object form, provided that You
      meet the following conditions:

      (a) You must give any other recipients of the Work or
          Derivative Works a copy of this License; and

      (b) You must cause any modified files to carry prominent notices
          stating that You changed the files; and

      (c) You must retain, in the Source form of any Derivative Works
          that You distribute, all copyright, patent, trademark, and
          attribution notices from the Source form of the Work,
          excluding those notices that do not pertain to any part of
          the Derivative Works; and

      (d) If the Work includes a "NOTICE" text file as part of its
          distribution, then any Derivative Works that You distribute must
          include a readable copy of the attribution notices contained
          within such NOTICE file, excluding those notices that do not
          pertain to any part of the Derivative Works, in at least one
          of the following places: within a NOTICE text file distributed
          as part of the Derivative Works; within the Source form or
          documentation, if provided along with the Derivative Works; or,
          within a display generated by the Derivative Works, if and
          wherever such third-party notices normally appear. The contents
          of the NOTICE file are for informational purposes only and
          do not modify the License. You may add Your own attribution
          notices within Derivative Works that You distribute, alongside
          or as an addendum to the NOTICE text from the Work, provided
          that such additional attribution notices cannot be construed
          as modifying the License.

      You may add Your own copyright statement to Your modifications and
      may provide additional or different license terms and conditions
      for use, reproduction, or distribution of Your modifications, or
      for any such Derivative Works as a whole, provided Your use,
      reproduction, and distribution of the Work otherwise complies with
      the conditions stated in this License.

   5. Submission of Contributions. Unless You explicitly state otherwise,
      any Contribution intentionally submitted for inclusion in the Work
      by You to the Licensor shall be under the terms and conditions of
      this License, without any additional terms or conditions.
      Notwithstanding the above, nothing herein shall supersede or modify
      the terms of any separate license agreement you may have executed
      with Licensor regarding such Contributions.

   6. Trademarks. This License does not grant permission to use the trade
      names, trademarks, service marks, or product names of the Licensor,
      except as required for reasonable and customary use in describing the
      origin of the Work and reproducing the content of the NOTICE file.

   7. Disclaimer of Warranty. Unless required by applicable law or
      agreed to in writing, Licensor provides the Work (and each
      Contributor provides its Contributions) on an "AS IS" BASIS,
      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
      implied, including, without limitation, any warranties or conditions
      of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
      PARTICULAR PURPOSE. You are solely responsible for determining the
      appropriateness of using or redistributing the Work and assume any
      risks associated with Your exercise of permissions under this License.

   8. Limitation of Liability. In no event and under no legal theory,
      whether in tort (including negligence), contract, or otherwise,
      unless required by applicable law (such as deliberate and grossly
      negligent acts) or agreed to in writing, shall any Contributor be
      liable to You for damages, including any direct, indirect, special,
      incidental, or consequential damages of any character arising as a
      result of this License or out of the use or inability to use the
      Work (including but not limited to damages for loss of goodwill,
      work stoppage, computer failure or malfunction, or any and all
      other commercial damages or losses), even if such Contributor
      has been advised of the possibility of such damages.

   9. Accepting Warranty or Additional Liability. While redistributing
      the Work or Derivative Works thereof, You may choose to offer,
      and charge a fee for, acceptance of support, warranty, indemnity,
      or other liability obligations and/or rights consistent with this
      License. However, in accepting such obligations, You may act only
      on Your own behalf and on Your sole responsibility, not on behalf
      of any other Contributor, and only if You agree to indemnify,
      defend, and hold each Contributor harmless for any liability
      incurred by, or claims asserted against, such Contributor by reason
      of your accepting any such warranty or additional liability.

   END OF TERMS AND CONDITIONS

   APPENDIX: How to apply the Apache License to your work.

      To apply the Apache License to your work, attach the following
      boilerplate notice, with the fields enclosed by brackets "[]"
      replaced with your own identifying information. (Don't include
      the brackets!)  The text should be enclosed in the appropriate
      comment syntax for the file format. We also recommend that a
      file or class name and description of purpose be included on the
      same "printed page" as the copyright notice for easier
      identification within third-party archives.

   Copyright [yyyy] [name of copyright owner]

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
```

</p></details>

- Mips instructions sourced from the [instruction set manual](https://www.cs.cornell.edu/courses/cs3410/2008fa/MIPS_Vol2.pdf)

- Mips registers sourced from [wikibooks](https://en.wikibooks.org/wiki/MIPS_Assembly/Register_File)

- Mars directives and pseudo ops sourced from the [project's repo](https://github.com/dpetersanderson/MARS)+

<details><summary>+ Licensed under MIT</summary><p>

```
Copyright (c) 2003-2013,  Pete Sanderson and Kenneth Vollmar

Developed by Pete Sanderson (psanderson@otterbein.edu)
and Kenneth Vollmar (kenvollmar@missouristate.edu)

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject
to the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR
ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF
CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

(MIT license, http://www.opensource.org/licenses/mit-license.html)
```

</p></details>

- z80 instructions and registers sourced from the [MSX documentation page](https://map.grauw.nl/resources/z80instr.php)
    and the [zilog user manual](https://www.zilog.com/docs/z80/z80cpu_um.pdf)
