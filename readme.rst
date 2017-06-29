.. image:: https://img.shields.io/github/license/Maratyszcza/Opcodes.svg
   :alt: License
   :target: https://github.com/Maratyszcza/Opcodes/blob/master/license.rst

.. image:: https://img.shields.io/pypi/v/opcodes.svg
   :alt: PyPI Package
   :target: https://pypi.python.org/pypi/opcodes

.. image:: https://readthedocs.org/projects/opcodes/badge/?style
   :alt: Documentation
   :target: https://opcodes.readthedocs.org

.. image:: https://img.shields.io/travis/Maratyszcza/Opcodes.svg
   :alt: Test Status
   :target: https://travis-ci.org/Maratyszcza/Opcodes

Opcodes Project
===============

The goal of this project is to document instruction sets in a format convenient for tools development. An instruction set is represented by three files:

- An XML file that describes instructions
- An XSD file that describes the structure of the XML file
- A Python module that reads the XML file and represents it as a set of Python objects

This project is a spin-off from `PeachPy <https://github.com/Maratyszcza/PeachPy>`_ assembler.

Current status
--------------

The project provides descriptions for most user-mode x86, x86-64, and k1om instructions up to AVX-512 and SHA (including 3dnow!+, XOP, FMA3, FMA4, TBM and BMI2). The following instructions are currently **NOT** supported:

- All priveledged instructions and user-mode system instructions (e.g. XSAVE, SLDT)
- Legacy string/streaming instructions (MOVS/SCAS/CMPS/STOS/LODS)
- LOCK and REP/REPZ/REPNZ prefixes
- x87 FPU instructions
- VIA Padlock instructions
- Intel HTM instructions

For each instruction the following information is provided:

- Summary description
- Instruction names in Intel assembly (`PeachPy <https://github.com/Maratyszcza/PeachPy>`_, `NASM <http://nasm.us>`_, `YASM <http://yasm.tortall.net>`_ and MASM assemblers), AT&T assembly (GNU assembler) and Plan 9 assembly (`Go <https://golang.org>`_ assembler)
- Operand types and characteristics (whether the operand is input or output)
- Implicit input and output registers
- ISA where this instruction was introduced
- Whether the instruction operates on FPU or MMX state
- Whether the instruction operates on AVX or legacy SSE state
- Whether the instruction has no dependency on input values when input operands refer to the same register (e.g. `XOR eax, eax` has no dependency on `eax`)
- Whether the instruction is supported by Native Client validator and when the support was introduced
- Whether the instruction that writes to a 32-bit register is recognized by x86-64 Native Client validator as zero-extending

Installation
------------

.. code-block:: bash

  pip install --upgrade Opcodes

Users
-----

- `PeachPy <https://github.com/Maratyszcza/PeachPy>`_ -- x86-64 assembler embedded in Python that targets High-Performance Computing use-cases.

- `Template-Assembly <https://github.com/mattbierner/Template-Assembly>`_ -- Embedding x86 assembly code in C++ with metaprogramming using a domain specific language.

Peer-Reviewed Publications
--------------------------

- Marat Dukhan "PeachPy meets Opcodes: Direct Machine Code Generation from Python", Python for High-Performance Computing (PyHPC) 2015 (`slides <http://www.peachpy.io/slides/pyhpc2015>`_, `paper on ACM Digital Library <https://dl.acm.org/citation.cfm?id=2835860>`_).

Acknowledgements
----------------

.. image:: https://github.com/Maratyszcza/PeachPy/blob/master/logo/hpcgarage.png
  :alt: HPC Garage logo
  :target: http://hpcgarage.org/

.. image:: https://github.com/Maratyszcza/PeachPy/blob/master/logo/college-of-computing.gif
  :alt: Georgia Tech College of Computing logo
  :target: http://www.cse.gatech.edu/

This work is a research project at the HPC Garage lab in the Georgia Institute of Technology, College of Computing, School of Computational Science and Engineering.

The work was supported in part by grants to Prof. Richard Vuduc's research lab, `The HPC Garage <www.hpcgarage.org>`_, from the National Science Foundation (NSF) under NSF CAREER award number 0953100; and a grant from the Defense Advanced Research Projects Agency (DARPA) Computer Science Study Group program

Any opinions, conclusions or recommendations expressed in this software and documentation are those of the authors and not necessarily reflect those of NSF or DARPA.
