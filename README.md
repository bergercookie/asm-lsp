# Language Server for GAS/GO Assembly

## Goal

Provide hovering and (TODO) autocompletion for assembly files written in the
GAS/NASM or GO assembly flavors. It supports assembly files for the x86 or
x86_64 instruction sets.

This tool can serve as reference when reading the assembly output of a program.
This way you can query what each command exactly does and deliberate about
whether the compiler is producing the desired output or whether you have to
tweak your code for optimisation.

## Demo

### Hovering / Documentation support

![](https://github.com/bergercookie/asm-lsp/blob/master/demo/hover.gif)

### Autocomplete

TODO

## Acknowledgements

Current rust package builds on top of the [opcodes python
package](https://github.com/Maratyszcza/Opcodes)
