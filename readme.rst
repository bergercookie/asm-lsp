.. image:: https://img.shields.io/badge/license-BSD-brightgreen.png
   :alt: License
   :target: https://github.com/Maratyszcza/Opcodes/blob/master/license.rst

.. image:: https://readthedocs.org/projects/opcodes/badge/?style
   :alt: Documentation
   :target: https://opcodes.readthedocs.org

Opcodes Project
===============

The goal of this project is to document instruction sets in a format convenient for tools development. An instruction set is represented by three files:

- An XML file that describes instructions
- An XSD file that describes the structure of the XML file
- A Python module that reads the XML file and represents it as a set of Python objects

This project is a spin-off from `Peach-Py <https://bitbucket.org/MDukhan/peachpy>`_ assembler.

Current status
--------------

The project provides descriptions for most user-mode x86-64 instructions up to AVX2, but excluding AMD and VIA extensions.

For each instruction the following information is provided:

- Summary description
- Instruction names in Intel assembly (`Peach-Py <https://bitbucket.org/MDukhan/peachpy>`_, `NASM <http://nasm.us>`_, `YASM <http://yasm.tortall.net>`_ and MASM assemblers), AT&T assembly (GNU assembler) and Plan 9 assembly (`Go <https://golang.org>`_ assembler)
- Operand types and characteristics (whether the operand is input or output)
- Implicit input and output registers
- ISA where this instruction was introduced
- Whether the instruction operates on FPU or MMX state
- Whether the instruction operates on AVX or legacy SSE state

