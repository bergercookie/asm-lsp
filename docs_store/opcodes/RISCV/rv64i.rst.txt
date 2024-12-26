RV64I Instructions
==================

addiw
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |imm[11:0]        |rs1  |000  |rd   |00110|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | addiw      rd,rs1,imm

:Description:
  | Adds the sign-extended 12-bit immediate to register rs1 and produces the proper sign-extension of a 32-bit result in rd.
  | Overflows are ignored and the result is the low 32 bits of the result sign-extended to 64 bits.
  | Note, ADDIW rd, rs1, 0 writes the sign-extension of the lower 32 bits of register rs1 into register rd (assembler pseudoinstruction SEXT.W).

:Implementation:
  | x[rd] = sext((x[rs1] + sext(immediate))[31:0])


slliw
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|00   |shamt|rs1  |001  |rd   |00110|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | slliw      rd,rs1,shamt

:Description:
  | Performs logical left shift on the 32-bit of value in register rs1 by the shift amount held in the lower 5 bits of the immediate.
  | Encodings with $imm[5] \neq 0$ are reserved.

:Implementation:
  | x[rd] = sext((x[rs1] << shamt)[31:0])


srliw
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|00   |shamt|rs1  |101  |rd   |00110|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | srliw      rd,rs1,shamt

:Description:
  | Performs logical right shift on the 32-bit of value in register rs1 by the shift amount held in the lower 5 bits of the immediate.
  | Encodings with $imm[5] \neq 0$ are reserved.

:Implementation:
  | x[rd] = sext(x[rs1][31:0] >>u shamt)



sraiw
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |01000|00   |shamt|rs1  |101  |rd   |00110|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | sraiw      rd,rs1,shamt

:Description:
  | Performs arithmetic right shift on the 32-bit of value in register rs1 by the shift amount held in the lower 5 bits of the immediate.
  | Encodings with $imm[5] \neq 0$ are reserved.

:Implementation:
  | x[rd] = sext(x[rs1][31:0] >>s shamt)



addw
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|00   |rs2  |rs1  |000  |rd   |01110|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | addw       rd,rs1,rs2

:Description:
  | Adds the 32-bit of registers rs1 and 32-bit of register rs2 and stores the result in rd.
  | Arithmetic overflow is ignored and the low 32-bits of the result is sign-extended to 64-bits and written to the destination register.


:Implementation:
  | x[rd] = sext((x[rs1] + x[rs2])[31:0])


subw
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |01000|00   |rs2  |rs1  |000  |rd   |01110|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | subw       rd,rs1,rs2

:Description:
  | Subtract the 32-bit of registers rs1 and 32-bit of register rs2 and stores the result in rd.
  | Arithmetic overflow is ignored and the low 32-bits of the result is sign-extended to 64-bits and written to the destination register.

:Implementation:
  | x[rd] = sext((x[rs1] - x[rs2])[31:0])


sllw
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|00   |rs2  |rs1  |001  |rd   |01110|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | sllw       rd,rs1,rs2

:Description:
  | Performs logical left shift on the low 32-bits value in register rs1 by the shift amount held in the lower 5 bits of register rs2 and produce 32-bit results and written to the destination register rd.

:Implementation:
  | x[rd] = sext((x[rs1] << x[rs2][4:0])[31:0])


srlw
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|00   |rs2  |rs1  |101  |rd   |01110|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | srlw       rd,rs1,rs2

:Description:
  | Performs logical right shift on the low 32-bits value in register rs1 by the shift amount held in the lower 5 bits of register rs2 and produce 32-bit results and written to the destination register rd.

:Implementation:
  | x[rd] = sext(x[rs1][31:0] >>u x[rs2][4:0])


sraw
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |01000|00   |rs2  |rs1  |101  |rd   |01110|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | sraw       rd,rs1,rs2

:Description:
  | Performs arithmetic right shift on the low 32-bits value in register rs1 by the shift amount held in the lower 5 bits of register rs2 and produce 32-bit results and written to the destination register rd.

:Implementation:
  | x[rd] = sext(x[rs1][31:0] >>s x[rs2][4:0])


lwu
----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |offset[11:0]     |rs1  |110  |rd   |00000|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | lwu        rd,offset(rs1)

:Description:
  | Loads a 32-bit value from memory and zero-extends this to 64 bits before storing it in register rd.

:Implementation:
  | x[rd] = M[x[rs1] + sext(offset)][31:0]



ld
---

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |offset[11:0]     |rs1  |011  |rd   |00000|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | ld         rd,offset(rs1)

:Description:
  | Loads a 64-bit value from memory into register rd for RV64I.

:Implementation:
  | x[rd] = M[x[rs1] + sext(offset)][63:0]


sd
---

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+------+-----+-----+-----+-----------+-----+---+
  |31-27|26-25 |24-20|19-15|14-12|11-7       |6-2  |1-0|
  +-----+------+-----+-----+-----+-----------+-----+---+
  |offset[11:5]|rs2  |rs1  |011  |offset[4:0]|01000|11 |
  +-----+------+-----+-----+-----+-----------+-----+---+



:Format:
  | sd         rs2,offset(rs1)

:Description:
  | Store 64-bit, values from register rs2 to memory.

:Implementation:
  | M[x[rs1] + sext(offset)] = x[rs2][63:0]
