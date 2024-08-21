RV32C, RV64C Instructions
=========================

c.addi4spn
-----------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +------+-------------------+-----+---+
  |15-13 |12-5               |4-2  |1-0|
  +------+-------------------+-----+---+
  |000   |nzuimm[5:4|9:6|2|3]|rd\' |00 |
  +------+-------------------+-----+---+


:Format:
  | c.addi4spn rd\',uimm

:Description:
  | Add a zero-extended non-zero immediate, scaled by 4, to the stack pointer, x2, and writes the result to rd\'.
  | This instruction is used to generate pointers to stack-allocated variables, and expands to addi rd\', x2, nzuimm[9:2].

:Implementation:
  | x[8+rd\'] = x[2] + nzuimm

:Expansion:
  | addi rd\',x2,nzuimm


c.fld
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+----------+-----+---------+----+---+
  |15-13|12-10     |9-7  |6-5      |4-2 |1-0|
  +-----+----------+-----+---------+----+---+
  |001  |uimm[5:3] |rs1\'|uimm[7:6]|rd\'|00 |
  +-----+----------+-----+---------+----+---+



:Format:
  | c.fld      rd\',uimm(rs1\')

:Description:
  | Load a double-precision floating-point value from memory into floating-point register rd\'.
  | It computes an effective address by adding the zero-extended offset, scaled by 8, to the base address in register rs1\'.

:Implementation:
  | f[8+rd\'] = M[x[8+rs1\'] + uimm][63:0]

:Expansion:
  | fld rd\',offset[7:3](rs1\')


c.lw
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+----------+-----+---------+----+---+
  |15-13|12-10     |9-7  |6-5      |4-2 |1-0|
  +-----+----------+-----+---------+----+---+
  |010  |uimm[5:3] |rs1\'|uimm[2|6]|rd\'|00 |
  +-----+----------+-----+---------+----+---+



:Format:
  | c.lw       rd\',uimm(rs1\')

:Description:
  | Load a 32-bit value from memory into register rd\'. It computes an effective address by adding the zero-extended offset, scaled by 4, to the base address in register rs1\'.

:Implementation:
  | x[8+rd\'] = sext(M[x[8+rs1\'] + uimm][31:0])
:Expansion:
  | lw rd\',offset[6:2](rs1\')


c.flw
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+----------+-----+---------+----+---+
  |15-13|12-10     |9-7  |6-5      |4-2 |1-0|
  +-----+----------+-----+---------+----+---+
  |011  |uimm[5:3] |rs1\'|uimm[2|6]|rd\'|00 |
  +-----+----------+-----+---------+----+---+



:Format:
  | c.flw      rd\',uimm(rs1\')

:Description:
  | Load a single-precision floating-point value from memory into floating-point register rd\'.
  | It computes an effective address by adding the zero-extended offset, scaled by 4, to the base address in register rs1\'.

:Implementation:
  | f[8+rd\'] = M[x[8+rs1\'] + uimm][31:0]

:Expansion:
  | lw rd\',offset[6:2](rs1\')

c.ld
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+----------+-----+---------+----+---+
  |15-13|12-10     |9-7  |6-5      |4-2 |1-0|
  +-----+----------+-----+---------+----+---+
  |011  |uimm[5:3] |rs1\'|uimm[7:6]|rd\'|00 |
  +-----+----------+-----+---------+----+---+



:Format:
  | c.ld       rd\',uimm(rs1\')

:Description:
  | Load a 64-bit value from memory into register rd\'.
  | It computes an effective address by adding the zero-extended offset, scaled by 8, to the base address in register rs1\'.

:Implementation:
  | x[8+rd\'] = M[x[8+rs1\'] + uimm][63:0]

:Expansion:
  | ld rd\', offset[7:3](rs1\')


c.fsd
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+----------+-----+---------+----+---+
  |15-13|12-10     |9-7  |6-5      |4-2 |1-0|
  +-----+----------+-----+---------+----+---+
  |101  |uimm[5:3] |rs1\'|uimm[7:6]|rd\'|00 |
  +-----+----------+-----+---------+----+---+



:Format:
  | c.fsd      rd\',uimm(rs1\')

:Description:
  | Store a double-precision floating-point value in floating-point register rs2\' to memory.
  | It computes an effective address by adding the zeroextended offset, scaled by 8, to the base address in register rs1\'.

:Implementation:
  | M[x[8+rs1\'] + uimm][63:0] = f[8+rs2\']

:Expansion:
  | fsd rs2\',offset[7:3](rs1\')


c.sw
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+----------+-----+---------+-----+---+
  |15-13|12-10     |9-7  |6-5      |4-2  |1-0|
  +-----+----------+-----+---------+-----+---+
  |110  |uimm[5:3] |rs1\'|uimm[2|6]|rs2\'|00 |
  +-----+----------+-----+---------+-----+---+


:Format:
  | c.sw       rd\',uimm(rs1\')

:Description:
  | Store a 32-bit value in register rs2\' to memory.
  | It computes an effective address by adding the zero-extended offset, scaled by 4, to the base address in register rs1\'.

:Implementation:
  | M[x[8+rs1\'] + uimm][31:0] = x[8+rs2\']

:Expansion:
  | sw rs2\',offset[6:2](rs1\')



c.fsw
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+----------+-----+---------+-----+---+
  |15-13|12-10     |9-7  |6-5      |4-2  |1-0|
  +-----+----------+-----+---------+-----+---+
  |111  |uimm[5:3] |rs1\'|uimm[2|6]|rs2\'|00 |
  +-----+----------+-----+---------+-----+---+


:Format:
  | c.fsw      rd\',uimm(rs1\')

:Description:
  | Store a single-precision floating-point value in floatingpoint register rs2\' to memory.
  | It computes an effective address by adding the zero-extended offset, scaled by 4, to the base address in register rs1\'.

:Implementation:
  | M[x[8+rs1\'] + uimm][31:0] = f[8+rs2\']

:Expansion:
  | fsw rs2\', offset[6:2](rs1\')



c.sd
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+----------+-----+---------+-----+---+
  |15-13|12-10     |9-7  |6-5      |4-2  |1-0|
  +-----+----------+-----+---------+-----+---+
  |111  |uimm[5:3] |rs1\'|uimm[7:6]|rs2\'|00 |
  +-----+----------+-----+---------+-----+---+



:Format:
  | c.sd       rd\',uimm(rs1\')

:Description:
  | Store a 64-bit value in register rs2\' to memory.
  | It computes an effective address by adding the zero-extended offset, scaled by 8, to the base address in register rs1\'.

:Implementation:
  | M[x[8+rs1\'] + uimm][63:0] = x[8+rs2\']

:Expansion:
  | sd rs2\', offset[7:3](rs1\')


c.nop
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+----------+-----+---------+-----+---+
  |15-13|12-10     |9-7  |6-5      |4-2  |1-0|
  +-----+----------+-----+---------+-----+---+
  |000  |0         |0    |0        |0    |01 |
  +-----+----------+-----+---------+-----+---+



:Format:
  | c.nop

:Description:
  | Does not change any user-visible state, except for advancing the pc.

:Implementation:
  | None

:Expansion:
  | addi x0, x0, 0


c.addi
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+----------+---------+----------+---+
  |15-13|12        |11-7     |6-2       |1-0|
  +-----+----------+---------+----------+---+
  |000  |nzimm[5]  |rs1/rd!=0|nzimm[4:0]|01 |
  +-----+----------+---------+----------+---+



:Format:
  | c.addi     rd,u[12:12]|u[6:2]

:Description:
  | Add the non-zero sign-extended 6-bit immediate to the value in register rd then writes the result to rd.

:Implementation:
  | x[rd] = x[rd] + sext(imm)

:Expansion:
  | addi rd, rd, nzimm[5:0]


c.jal
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-------------------------------+---+
  |15-13|12-2                           |1-0|
  +-----+-------------------------------+---+
  |001  |imm[11|4|9:8|10|6|7|3:1|5]     |01 |
  +-----+-------------------------------+---+


:Format:
  | c.jal offset

:Description:
  | Jump to address and place return address in rd.

:Implementation:
  | x[1] = pc+2; pc += sext(offset)

:Expansion:
  | jal x1, offset[11:1]


c.addiw
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+------+-----+--------+---+
  |15-13|12    |11-7 |6-2     |1-0|
  +-----+------+-----+--------+---+
  |001  |imm[5]|rd   |imm[4:0]|01 |
  +-----+------+-----+--------+---+


:Format:
  | c.addiw    rd,imm

:Description:
  | Add the non-zero sign-extended 6-bit immediate to the value in register rd then produce 32-bit result, then sign-extends result to 64 bits.

:Implementation:
  | x[rd] = sext((x[rd] + sext(imm))[31:0])

:Expansion:
  | addiw rd,rd,imm[5:0]


c.li
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+------+-----+--------+---+
  |15-13|12    |11-7 |6-2     |1-0|
  +-----+------+-----+--------+---+
  |010  |imm[5]|rd   |imm[4:0]|01 |
  +-----+------+-----+--------+---+


:Format:
  | c.li       rd,imm

:Description:
  | Load the sign-extended 6-bit immediate, imm, into register rd.
  | C.LI is only valid when rd!=x0.

:Implementation:
  | x[rd] = sext(imm)

:Expansion:
  | addi rd,x0,imm[5:0]


c.addi16sp
-----------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+------+-----+--------------+---+
  |15-13|12    |11-7 |6-2           |1-0|
  +-----+------+-----+--------------+---+
  |011  |imm[9]|00010|imm[4|6|8:7|5]|01 |
  +-----+------+-----+--------------+---+



:Format:
  | c.addi16sp imm

:Description:
  | Add the non-zero sign-extended 6-bit immediate to the value in the stack pointer (sp=x2), where the immediate is scaled to represent multiples of 16 in the range (-512,496).

:Implementation:
  | x[2] = x[2] + sext(imm)

:Expansion:
  | addi x2,x2, nzimm[9:4]


c.lui
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-------+-----+----------+---+
  |15-13|12     |11-7 |6-2       |1-0|
  +-----+-------+-----+----------+---+
  |011  |imm[17]|rd   |imm[16:12]|01 |
  +-----+-------+-----+----------+---+


:Format:
  | c.lui      rd,imm

:Description:
  |
:Implementation:
  | x[rd] = sext(imm[17:12] << 12)

:Expansion:
  | lui rd,nzuimm[17:12]


c.srli
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-------+-----+----+---------+---+
  |15-13|12     |11-10|9-7 |6-2      |1-0|
  +-----+-------+-----+----+---------+---+
  |100  |uimm[5]|00   |rd\'|uimm[4:0]|01 |
  +-----+-------+-----+----+---------+---+


:Format:
  | c.srli     rd\',uimm

:Description:
  | Perform a logical right shift of the value in register rd\' then writes the result to rd\'.
  | The shift amount is encoded in the shamt field, where shamt[5] must be zero for RV32C.

:Implementation:
  | x[8+rd\'] = x[8+rd\'] >>u uimm

:Expansion:
  | srli rd\',rd\',shamt[5:0]



c.srai
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-------+-----+----+---------+---+
  |15-13|12     |11-10|9-7 |6-2      |1-0|
  +-----+-------+-----+----+---------+---+
  |100  |uimm[5]|01   |rd\'|uimm[4:0]|01 |
  +-----+-------+-----+----+---------+---+


:Format:
  | c.srai     rd\',uimm

:Description:
  | Perform a arithmetic right shift of the value in register rd\' then writes the result to rd\'.
  | The shift amount is encoded in the shamt field, where shamt[5] must be zero for RV32C.

:Implementation:
  | x[8+rd\'] = x[8+rd\'] >>s uimm

:Expansion:
  | srai rd\',rd\',shamt[5:0]



c.andi
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+------+-----+----+--------+---+
  |15-13|12    |11-10|9-7 |6-2     |1-0|
  +-----+------+-----+----+--------+---+
  |100  |imm[5]|10   |rd\'|imm[4:0]|01 |
  +-----+------+-----+----+--------+---+



:Format:
  | c.andi     rd\',imm

:Description:
  | Compute the bitwise AND of of the value in register rd\' and the sign-extended 6-bit immediate, then writes the result to rd\'.

:Implementation:
  | x[8+rd\'] = x[8+rd\'] & sext(imm)

:Expansion:
  | andi rd\',rd\',imm[5:0]


c.sub
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +------+----+---+-----+---+
  |15-10 |9-7 |6-5|4-2  |1-0|
  +------+----+---+-----+---+
  |100011|rd\'|00 |rs2\'|01 |
  +------+----+---+-----+---+


:Format:
  | c.sub      rd\',rs2\'

:Description:
  | Subtract the value in register rs2\' from the value in register rd\', then writes the result to register rd\'.

:Implementation:
  | x[8+rd\'] = x[8+rd\'] - x[8+rs2\']

:Expansion:
  | sub rd\',rd\',rs2\'


c.xor
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +------+----+---+-----+---+
  |15-10 |9-7 |6-5|4-2  |1-0|
  +------+----+---+-----+---+
  |100011|rd\'|01 |rs2\'|01 |
  +------+----+---+-----+---+



:Format:
  | c.xor      rd\',rs2\'

:Description:
  | Compute the bitwise XOR of the values in registers rd\' and rs2\', then writes the result to register rd\'.

:Implementation:
  | x[8+rd\'] = x[8+rd\'] ^ x[8+rs2\']

:Expansion:
  | xor rd\',rd\',rs2\'


c.or
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +------+----+---+-----+---+
  |15-10 |9-7 |6-5|4-2  |1-0|
  +------+----+---+-----+---+
  |100011|rd\'|10 |rs2\'|01 |
  +------+----+---+-----+---+



:Format:
  | c.or       rd\',rs2\'

:Description:
  | Compute the bitwise OR of the values in registers rd\' and rs2\', then writes the result to register rd\'.

:Implementation:
  | x[8+rd\'] = x[8+rd\'] | x[8+rs2\']

:Expansion:
  |  or rd\',rd\',rs2


c.and
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +------+----+---+-----+---+
  |15-10 |9-7 |6-5|4-2  |1-0|
  +------+----+---+-----+---+
  |100011|rd\'|11 |rs2\'|01 |
  +------+----+---+-----+---+



:Format:
  | c.and      rd\',rs2\'

:Description:
  | Compute the bitwise AND of the values in registers rd\' and rs2\', then writes the result to register rd\'.

:Implementation:
  | x[8+rd\'] = x[8+rd\'] & x[8+rs2\']

:Expansion:
  | and rd\',rd\',rs2\'



c.subw
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +------+----+---+-----+---+
  |15-10 |9-7 |6-5|4-2  |1-0|
  +------+----+---+-----+---+
  |100111|rd\'|00 |rs2\'|01 |
  +------+----+---+-----+---+



:Format:
  | c.subw     rd\',rs2\'

:Description:
  | Subtract the value in register rs2\' from the value in register rd\', then sign-extends the lower 32 bits of the difference before writing the result to register rd\'.

:Implementation:
  | x[8+rd\'] = sext((x[8+rd\'] - x[8+rs2\'])[31:0])

:Expansion:
  | subw rd\',rd\',rs2\'


c.addw
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +------+----+---+-----+---+
  |15-10 |9-7 |6-5|4-2  |1-0|
  +------+----+---+-----+---+
  |100111|rd\'|01 |rs2\'|01 |
  +------+----+---+-----+---+



:Format:
  | c.addw     rd\',rs2\'

:Description:
  | Add the value in register rs2\' from the value in register rd\', then sign-extends the lower 32 bits of the difference before writing the result to register rd\'.

:Implementation:
  | x[8+rd\'] = sext((x[8+rd\'] + x[8+rs2\'])[31:0])

:Expansion:
  | addw rd\',rd\',rs2\'


c.j
----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-------------------------------+---+
  |15-13|12-2                           |1-0|
  +-----+-------------------------------+---+
  |101  |imm[11|4|9:8|10|6|7|3:1|5]     |01 |
  +-----+-------------------------------+---+



:Format:
  | c.j offset

:Description:
  | Unconditional control transfer.

:Implementation:
  | pc += sext(offset)

:Expansion:
  | jal x0,offset[11:1]


c.beqz
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-------------+-----+-----------------+---+
  |15-13|12-10        |9-7  |6-2              |1-0|
  +-----+-------------+-----+-----------------+---+
  |110  |offset[8|4:3]|rs1\'|offset[7:6|2:1|5]|01 |
  +-----+-------------+-----+-----------------+---+


:Format:
  | c.beqz     rs1\',offset

:Description:
  | Take the branch if the value in register rs1\' is zero.

:Implementation:
  | if (x[8+rs1\'] == 0) pc += sext(offset)

:Expansion:
  | beq rs1\',x0,offset[8:1]


c.bnez
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-------------+-----+-----------------+---+
  |15-13|12-10        |9-7  |6-2              |1-0|
  +-----+-------------+-----+-----------------+---+
  |111  |offset[8|4:3]|rs1\'|offset[7:6|2:1|5]|01 |
  +-----+-------------+-----+-----------------+---+


:Format:
  | c.bnez     rs1\',offset

:Description:
  | Take the branch if the value in register rs1\' is not zero.

:Implementation:
  | if (x[8+rs1\'] != 0) pc += sext(offset)

:Expansion:
  | bne rs1\',x0,offset[8:1]


c.slli
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-------+-----+----------+---+
  |15-13|12     |11-7 |6-2       |1-0|
  +-----+-------+-----+----------+---+
  |010  |uimm[5]|rd   |uimm[4:0] |10 |
  +-----+-------+-----+----------+---+



:Format:
  | c.slli     rd,uimm

:Description:
  | Perform a logical left shift of the value in register rd then writes the result to rd.
  | The shift amount is encoded in the shamt field, where shamt[5] must be zero for RV32C.

:Implementation:
  | x[rd] = x[rd] << uimm

:Expansion:
  | slli rd,rd,shamt[5:0]


c.fldsp
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-------+-----+--------------+---+
  |15-13|12     |11-7 |6-2           |1-0|
  +-----+-------+-----+--------------+---+
  |001  |uimm[5]|rd   |uimm[4:3|8:6] |10 |
  +-----+-------+-----+--------------+---+


:Format:
  | c.fldsp    rd,uimm(x2)

:Description:
  | Load a double-precision floating-point value from memory into floating-point register rd.
  | It computes its effective address by adding the zero-extended offset, scaled by 8, to the stack pointer, x2.

:Implementation:
  | f[rd] = M[x[2] + uimm][63:0]

:Expansion:
  | fld rd,offset[8:3](x2)



c.lwsp
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-------+-----+--------------+---+
  |15-13|12     |11-7 |6-2           |1-0|
  +-----+-------+-----+--------------+---+
  |010  |uimm[5]|rd   |uimm[4:2|7:6] |10 |
  +-----+-------+-----+--------------+---+


:Format:
  | c.lwsp     rd,uimm(x2)

:Description:
  | Load a 32-bit value from memory into register rd. It computes an effective address by adding the zero-extended offset, scaled by 4, to the stack pointer, x2.

:Implementation:
  | x[rd] = sext(M[x[2] + uimm][31:0])

:Expansion:
  | lw rd,offset[7:2](x2)



c.flwsp
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-------+-----+--------------+---+
  |15-13|12     |11-7 |6-2           |1-0|
  +-----+-------+-----+--------------+---+
  |011  |uimm[5]|rd   |uimm[4:2|7:6] |10 |
  +-----+-------+-----+--------------+---+



:Format:
  | c.flwsp    rd,uimm(x2)

:Description:
  | Load a single-precision floating-point value from memory into floating-point register rd.
  | It computes its effective address by adding the zero-extended offset, scaled by 4, to the stack pointer, x2.

:Implementation:
  | f[rd] = M[x[2] + uimm][31:0]

:Expansion:
  | flw rd,offset[7:2](x2)



c.ldsp
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-------+-----+--------------+---+
  |15-13|12     |11-7 |6-2           |1-0|
  +-----+-------+-----+--------------+---+
  |011  |uimm[5]|rd   |uimm[4:3|8:6] |10 |
  +-----+-------+-----+--------------+---+



:Format:
  | c.ldsp     rd,uimm(x2)

:Description:
  | Load a 64-bit value from memory into register rd.
  | It computes its effective address by adding the zero-extended offset, scaled by 8, to the stack pointer, x2.

:Implementation:
  | x[rd] = M[x[2] + uimm][63:0]

:Expansion:
  | ld rd,offset[8:3](x2)



c.jr
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-------+-----+------+---+
  |15-13|12     |11-7 |6-2   |1-0|
  +-----+-------+-----+------+---+
  |100  |0      |rs1  |00000 |10 |
  +-----+-------+-----+------+---+


:Format:
  | c.jr rs1

:Description:
  | Performs an unconditional control transfer to the address in register rs1.

:Implementation:
  | pc = x[rs1]

:Expansion:
  | jalr x0,rs1,0


c.mv
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-------+-----+------+---+
  |15-13|12     |11-7 |6-2   |1-0|
  +-----+-------+-----+------+---+
  |100  |0      |rd   |rs2   |10 |
  +-----+-------+-----+------+---+



:Format:
  | c.mv       rd,rs2

:Description:
  | Copy the value in register rs2 into register rd.

:Implementation:
  | x[rd] = x[rs2]

:Expansion:
  | add rd, x0, rs2


c.ebreak
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+---+-----+------+---+
  |15-13|12 |11-7 |6-2   |1-0|
  +-----+---+-----+------+---+
  |100  |1  |00000|00000 |10 |
  +-----+---+-----+------+---+




:Format:
  | c.ebreak

:Description:
  | Cause control to be transferred back to the debugging environment.

:Implementation:
  | RaiseException(Breakpoint)

:Expansion:
  | ebreak


c.jalr
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+---+-----+------+---+
  |15-13|12 |11-7 |6-2   |1-0|
  +-----+---+-----+------+---+
  |100  |1  |rs1  |00000 |10 |
  +-----+---+-----+------+---+


:Format:
  | c.jalr     rd

:Description:
  | Jump to address and place return address in rd.

:Implementation:
  | t = pc+2; pc = x[rs1]; x[1] = t

:Expansion:
  | jalr x1,rs1,0



c.add
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+---+-----+------+---+
  |15-13|12 |11-7 |6-2   |1-0|
  +-----+---+-----+------+---+
  |100  |1  |rd   |rs2   |10 |
  +-----+---+-----+------+---+



:Format:
  | c.add      rd,rs2

:Description:
  | Add the values in registers rd and rs2 and writes the result to register rd.

:Implementation:
  | x[rd] = x[rd] + x[rs2]

:Expansion:
  | add rd,rd,rs2


c.fsdsp
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--------------+-----+---+
  |15-13|12-7          |6-2  |1-0|
  +-----+--------------+-----+---+
  |101  |uimm[5:3|8:6] |rs2  |10 |
  +-----+--------------+-----+---+



:Format:
  | c.fsdsp rs2,uimm(x2)

:Description:
  | Store a double-precision floating-point value in floating-point register rs2 to memory.
  | It computes an effective address by adding the zeroextended offset, scaled by 8, to the stack pointer, x2.

:Implementation:
  | M[x[2] + uimm][63:0] = f[rs2]

:Expansion:
  | fsd rs2,offset[8:3](x2)


c.swsp
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--------------+-----+---+
  |15-13|12-7          |6-2  |1-0|
  +-----+--------------+-----+---+
  |110  |uimm[5:2|7:6] |rs2  |10 |
  +-----+--------------+-----+---+


:Format:
  | c.swsp rs2,uimm(x2)

:Description:
  | Store a 32-bit value in register rs2 to memory.
  | It computes an effective address by adding the zero-extended offset, scaled by 4, to the stack pointer, x2.

:Implementation:
  | M[x[2] + uimm][31:0] = x[rs2]

:Expansion:
  | sw rs2,offset[7:2](x2)


c.fswsp
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--------------+-----+---+
  |15-13|12-7          |6-2  |1-0|
  +-----+--------------+-----+---+
  |111  |uimm[5:2|7:6] |rs2  |10 |
  +-----+--------------+-----+---+


:Format:
  | c.fswsp rs2,uimm(rs2)

:Description:
  | Store a single-precision floating-point value in floating-point register rs2 to memory.
  | It computes an effective address by adding the zero-extended offset, scaled by 4, to the stack pointer, x2.

:Implementation:
  | M[x[2] + uimm][31:0] = f[rs2]

:Expansion:
  | fsw rs2,offset[7:2](x2)


c.sdsp
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--------------+-----+---+
  |15-13|12-7          |6-2  |1-0|
  +-----+--------------+-----+---+
  |111  |uimm[5:3|8:6] |rs2  |10 |
  +-----+--------------+-----+---+



:Format:
  | c.sdsp rs2,uimm(x2)

:Description:
  | Store a 64-bit value in register rs2 to memory.
  | It computes an effective address by adding the zero-extended offset, scaled by 8, to the stack pointer, x2.

:Implementation:
  | M[x[2] + uimm][63:0] = x[rs2]

:Expansion:
  | sd rs2,offset[8:3](x2)
