RV32F, RV64D Instructions
=========================

fmadd.s
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |rs3  |00   |rs2  |rs1  |rm   |rd   |10000|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+


:Format:
  | fmadd.s    rd,rs1,rs2,rs3

:Description:
  | Perform single-precision fused multiply addition.

:Implementation:
  | f[rd] = f[rs1]×f[rs2]+f[rs3]


fmsub.s
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |rs3  |00   |rs2  |rs1  |rm   |rd   |10001|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fmsub.s    rd,rs1,rs2,rs3

:Description:
  | Perform single-precision fused multiply addition.

:Implementation:
  | f[rd] = f[rs1]×f[rs2]-f[rs3]



fnmsub.s
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |rs3  |00   |rs2  |rs1  |rm   |rd   |10010|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fnmsub.s   rd,rs1,rs2,rs3

:Description:
  | Perform single-precision fused multiply addition.

:Implementation:
  | f[rd] = -f[rs1]×f[rs2]+f[rs3]


fnmadd.s
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |rs3  |00   |rs2  |rs1  |rm   |rd   |10011|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fnmadd.s   rd,rs1,rs2,rs3

:Description:
  | Perform single-precision fused multiply addition.

:Implementation:
  | f[rd] = -f[rs1]×f[rs2]-f[rs3]




fadd.s
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|00   |rs2  |rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fadd.s     rd,rs1,rs2

:Description:
  | Perform single-precision floating-point addition.

:Implementation:
  | f[rd] = f[rs1] + f[rs2]


fsub.s
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00001|00   |rs2  |rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fsub.s     rd,rs1,rs2

:Description:
  | Perform single-precision floating-point substraction.

:Implementation:
  | f[rd] = f[rs1] - f[rs2]




fmul.s
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00010|00   |rs2  |rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fmul.s     rd,rs1,rs2

:Description:
  | Perform single-precision floating-point multiplication.

:Implementation:
  | f[rd] = f[rs1] × f[rs2]




fdiv.s
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00011|00   |rs2  |rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fdiv.s     rd,rs1,rs2

:Description:
  | Perform single-precision floating-point division.

:Implementation:
  | f[rd] = f[rs1] / f[rs2]




fsqrt.s
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |01011|00   |00000|rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fsqrt.s    rd,rs1

:Description:
  | Perform single-precision square root.

:Implementation:
  | f[rd] = sqrt(f[rs1])




fsgnj.s
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00100|00   |rs2  |rs1  |000  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fsgnj.s    rd,rs1,rs2

:Description:
  | Produce a result that takes all bits except the sign bit from rs1.
  | The result’s sign bit is rs2’s sign bit.

:Implementation:
  | f[rd] = {f[rs2][31], f[rs1][30:0]}


fsgnjn.s
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00100|00   |rs2  |rs1  |001  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fsgnjn.s   rd,rs1,rs2

:Description:
  | Produce a result that takes all bits except the sign bit from rs1.
  | The result’s sign bit is opposite of rs2’s sign bit.

:Implementation:
  | f[rd] = {~f[rs2][31], f[rs1][30:0]}




fsgnjx.s
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00100|00   |rs2  |rs1  |010  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fsgnjx.s   rd,rs1,rs2

:Description:
  | Produce a result that takes all bits except the sign bit from rs1.
  | The result’s sign bit is XOR of sign bit of rs1 and rs2.

:Implementation:
  | f[rd] = {f[rs1][31] ^ f[rs2][31], f[rs1][30:0]}




fmin.s
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00101|00   |rs2  |rs1  |000  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fmin.s     rd,rs1,rs2

:Description:
  | Write the smaller of single precision data in rs1 and rs2 to rd.

:Implementation:
  | f[rd] = min(f[rs1], f[rs2])


fmax.s
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00101|00   |rs2  |rs1  |001  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fmax.s     rd,rs1,rs2

:Description:
  | Write the larger of single precision data in rs1 and rs2 to rd.

:Implementation:
  | f[rd] = max(f[rs1], f[rs2])


fcvt.w.s
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |11000|00   |00000|rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fcvt.w.s   rd,rs1

:Description:
  | Convert a floating-point number in floating-point register rs1 to a signed 32-bit in integer register rd.

:Implementation:
  | x[rd] = sext(s32_{f32}(f[rs1]))


fcvt.wu.s
----------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |11000|00   |00001|rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fcvt.wu.s  rd,rs1

:Description:
  | Convert a floating-point number in floating-point register rs1 to a signed 32-bit in unsigned integer register rd.

:Implementation:
  | x[rd] = sext(u32_{f32}(f[rs1]))


fmv.x.w
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |11100|00   |00000|rs1  |000  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fmv.x.w    rd,rs1

:Description:
  | Move the single-precision value in floating-point register rs1 represented in IEEE 754-2008 encoding to the lower 32 bits of integer register rd.

:Implementation:
  | x[rd] = sext(f[rs1][31:0])


feq.s
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |10100|00   |rs2  |rs1  |010  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | feq.s      rd,rs1,rs2

:Description:
  | Performs a quiet equal comparison between floating-point registers rs1 and rs2 and record the Boolean result in integer register rd.
  | Only signaling NaN inputs cause an Invalid Operation exception.
  | The result is 0 if either operand is NaN.

:Implementation:
  | x[rd] = f[rs1] == f[rs2]


flt.s
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |10100|00   |rs2  |rs1  |001  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | flt.s      rd,rs1,rs2

:Description:
  | Performs a quiet less comparison between floating-point registers rs1 and rs2 and record the Boolean result in integer register rd.
  | Only signaling NaN inputs cause an Invalid Operation exception.
  | The result is 0 if either operand is NaN.

:Implementation:
  | x[rd] = f[rs1] < f[rs2]



fle.s
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |10100|00   |rs2  |rs1  |000  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fle.s      rd,rs1,rs2

:Description:
  | Performs a quiet less or equal comparison between floating-point registers rs1 and rs2 and record the Boolean result in integer register rd.
  | Only signaling NaN inputs cause an Invalid Operation exception.
  | The result is 0 if either operand is NaN.

:Implementation:
  | x[rd] = f[rs1] <= f[rs2]



fclass.s
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |11100|00   |00000|rs1  |001  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fclass.s   rd,rs1

:Description:
  | Examines the value in floating-point register rs1 and writes to integer register rd a 10-bit mask that indicates the class of the floating-point number.
  | The format of the mask is described in [classify table]_.
  | The corresponding bit in rd will be set if the property is true and clear otherwise.
  | All other bits in rd are cleared. Note that exactly one bit in rd will be set.

:Implementation:
  | x[rd] = classifys(f[rs1])



fcvt.s.w
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |11010|00   |00000|rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fcvt.s.w   rd,rs1

:Description:
  | Converts a 32-bit signed integer, in integer register rs1 into a floating-point number in floating-point register rd.

:Implementation:
  | f[rd] = f32_{s32}(x[rs1])


fcvt.s.wu
----------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |11010|00   |00001|rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fcvt.s.wu  rd,rs1

:Description:
  | Converts a 32-bit unsigned integer, in integer register rs1 into a floating-point number in floating-point register rd.

:Implementation:
  | f[rd] = f32_{u32}(x[rs1])


fmv.w.x
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |11110|00   |00000|rs1  |000  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fmv.w.x    rd,rs1

:Description:
  | Move the single-precision value encoded in IEEE 754-2008 standard encoding from the lower 32 bits of integer register rs1 to the floating-point register rd.

:Implementation:
  | f[rd] = x[rs1][31:0]


fmadd.d
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |rs3  |01   |rs2  |rs1  |rm   |rd   |10000|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+


:Format:
  | fmadd.d    rd,rs1,rs2,rs3

:Description:
  | Perform single-precision fused multiply addition.

:Implementation:
  | f[rd] = f[rs1]×f[rs2]+f[rs3]


fmsub.d
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |rs3  |01   |rs2  |rs1  |rm   |rd   |10001|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fmsub.d    rd,rs1,rs2,rs3

:Description:
  | Perform single-precision fused multiply addition.

:Implementation:
  | f[rd] = f[rs1]×f[rs2]-f[rs3]



fnmsub.d
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |rs3  |01   |rs2  |rs1  |rm   |rd   |10010|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fnmsub.d   rd,rs1,rs2,rs3

:Description:
  | Perform single-precision fused multiply addition.

:Implementation:
  | f[rd] = -f[rs1]×f[rs2+f[rs3]



fnmadd.d
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |rs3  |01   |rs2  |rs1  |rm   |rd   |10011|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fnmadd.d   rd,rs1,rs2,rs3

:Description:
  | Perform single-precision fused multiply addition.

:Implementation:
  | f[rd] = -f[rs1]×f[rs2]-f[rs3]


fadd.d
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|01   |rs2  |rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fadd.d     rd,rs1,rs2

:Description:
  | Perform single-precision floating-point addition.

:Implementation:
  | f[rd] = f[rs1] + f[rs2]


fsub.d
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00001|01   |rs2  |rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fsub.d     rd,rs1,rs2

:Description:
  | Perform single-precision floating-point subtraction.

:Implementation:
  | f[rd] = f[rs1] - f[rs2]


fmul.d
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00010|01   |rs2  |rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fmul.d     rd,rs1,rs2

:Description:
  | Perform single-precision floating-point multiplication.

:Implementation:
  | f[rd] = f[rs1] × f[rs2]



fdiv.d
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00011|01   |rs2  |rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fdiv.d     rd,rs1,rs2

:Description:
  | Perform single-precision floating-point division.

:Implementation:
  | f[rd] = f[rs1] / f[rs2]



fsqrt.d
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |01011|01   |00000|rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fsqrt.d    rd,rs1

:Description:
  | Perform single-precision square root.

:Implementation:
  | f[rd] = sqrt(f[rs1])


fsgnj.d
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00100|01   |rs2  |rs1  |000  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fsgnj.d    rd,rs1,rs2

:Description:
  | Produce a result that takes all bits except the sign bit from rs1.
  | The result’s sign bit is rs2’s sign bit.

:Implementation:
  | f[rd] = {f[rs2][63], f[rs1][62:0]}



fsgnjn.d
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00100|01   |rs2  |rs1  |001  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fsgnjn.d   rd,rs1,rs2

:Description:
  | Produce a result that takes all bits except the sign bit from rs1.
  | The result’s sign bit is opposite of rs2’s sign bit.

:Implementation:
  | f[rd] = {~f[rs2][63], f[rs1][62:0]}


fsgnjx.d
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00100|01   |rs2  |rs1  |010  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fsgnjx.d   rd,rs1,rs2

:Description:
  | Produce a result that takes all bits except the sign bit from rs1.
  | The result’s sign bit is XOR of sign bit of rs1 and rs2.

:Implementation:
  | f[rd] = {f[rs1][63] ^ f[rs2][63], f[rs1][62:0]}



fmin.d
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00101|01   |rs2  |rs1  |000  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fmin.d     rd,rs1,rs2

:Description:
  | Write the smaller of single precision data in rs1 and rs2 to rd.

:Implementation:
  | f[rd] = min(f[rs1], f[rs2])


fmax.d
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00101|01   |rs2  |rs1  |001  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fmax.d     rd,rs1,rs2

:Description:
  | Write the larger of single precision data in rs1 and rs2 to rd.

:Implementation:
  | f[rd] = max(f[rs1], f[rs2])


fcvt.s.d
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |01000|00   |00001|rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fcvt.s.d   rd,rs1

:Description:
  | Converts double floating-point register in rs1 into a floating-point number in floating-point register rd.

:Implementation:
  | f[rd] = f32_{f64}(f[rs1])


fcvt.d.s
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |01000|01   |00000|rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+


:Format:
  | fcvt.d.s   rd,rs1

:Description:
  | Converts single floating-point register in rs1 into a double floating-point number in floating-point register rd.

:Implementation:
  | f[rd] = f64_{f32}(f[rs1])


feq.d
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |10100|01   |rs2  |rs1  |010  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | feq.d      rd,rs1,rs2

:Description:
  | Performs a quiet equal comparison between floating-point registers rs1 and rs2 and record the Boolean result in integer register rd.
  | Only signaling NaN inputs cause an Invalid Operation exception.
  | The result is 0 if either operand is NaN.

:Implementation:
  | x[rd] = f[rs1] == f[rs2]


flt.d
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |10100|01   |rs2  |rs1  |001  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | flt.d      rd,rs1,rs2

:Description:
  | Performs a quiet less comparison between floating-point registers rs1 and rs2 and record the Boolean result in integer register rd.
  | Only signaling NaN inputs cause an Invalid Operation exception.
  | The result is 0 if either operand is NaN.


:Implementation:
  | x[rd] = f[rs1] < f[rs2]



fle.d
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |10100|01   |rs2  |rs1  |000  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fle.d      rd,rs1,rs2

:Description:
  | Performs a quiet less or equal comparison between floating-point registers rs1 and rs2 and record the Boolean result in integer register rd.
  | Only signaling NaN inputs cause an Invalid Operation exception.
  | The result is 0 if either operand is NaN.

:Implementation:
  | x[rd] = f[rs1] <= f[rs2]


fclass.d
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |11100|01   |00000|rs1  |001  |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fclass.d   rd,rs1

:Description:
  | Examines the value in floating-point register rs1 and writes to integer register rd a 10-bit mask that indicates the class of the floating-point number.
  | The format of the mask is described in table [classify table]_.
  | The corresponding bit in rd will be set if the property is true and clear otherwise.
  | All other bits in rd are cleared. Note that exactly one bit in rd will be set.

:Implementation:
  | x[rd] = classifys(f[rs1])



fcvt.w.d
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |11000|01   |00000|rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fcvt.w.d   rd,rs1

:Description:
  | Converts a double-precision floating-point number in floating-point register rs1 to a signed 32-bit integer, in integer register rd.

:Implementation:
  | x[rd] = sext(s32_{f64}(f[rs1]))



fcvt.wu.d
----------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |11000|01   |00001|rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fcvt.wu.d  rd,rs1

:Description:
  | Converts a double-precision floating-point number in floating-point register rs1 to a unsigned 32-bit integer, in integer register rd.

:Implementation:
  | x[rd] = sext(u32f64(f[rs1]))



fcvt.d.w
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |11010|01   |00000|rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fcvt.d.w   rd,rs1

:Description:
  | Converts a 32-bit signed integer, in integer register rs1 into a double-precision floating-point number in floating-point register rd.

:Implementation:
  | x[rd] = sext(s32_{f64}(f[rs1]))


fcvt.d.wu
----------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |11010|01   |00001|rs1  |rm   |rd   |10100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fcvt.d.wu  rd,rs1

:Description:
  | Converts a 32-bit unsigned integer, in integer register rs1 into a double-precision floating-point number in floating-point register rd.

:Implementation:
  | f[rd] = f64_{u32}(x[rs1])


flw
----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |imm[11:0]        |rs1  |010  |rd   |00001|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | flw        rd,offset(rs1)

:Description:
  | Load a single-precision floating-point value from memory into floating-point register rd.

:Implementation:
  | f[rd] = M[x[rs1] + sext(offset)][31:0]


fsw
----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+--------+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7    |6-2  |1-0|
  +-----+-----+-----+-----+-----+--------+-----+---+
  |imm[11:5]  |rs2  |rs1  |010  |imm[4:0]|01001|11 |
  +-----+-----+-----+-----+-----+--------+-----+---+



:Format:
  | fsw        rs2,offset(rs1)

:Description:
  | Store a single-precision value from floating-point register rs2 to memory.

:Implementation:
  | M[x[rs1] + sext(offset)] = f[rs2][31:0]


fld
----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |imm[11:0]        |rs1  |011  |rd   |00001|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | fld        rd,rs1,offset

:Description:
  | Load a double-precision floating-point value from memory into floating-point register rd.

:Implementation:
  | f[rd] = M[x[rs1] + sext(offset)][63:0]


fsd
----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+--------+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7    |6-2  |1-0|
  +-----+-----+-----+-----+-----+--------+-----+---+
  |imm[11:5]  |rs2  |rs1  |011  |imm[4:0]|01001|11 |
  +-----+-----+-----+-----+-----+--------+-----+---+



:Format:
  | fsd        rs2,offset(rs1)

:Description:
  | Store a double-precision value from the floating-point registers to memory.

:Implementation:
  | M[x[rs1] + sext(offset)] = f[rs2][63:0]


.. [classify table]
.. table::
  Classify Table:

  +--------+------------------------------------+
  | rd bit | Meaning                            |
  +========+====================================+
  | 0      | rs1 is -infinity                   |
  +--------+------------------------------------+
  | 1      | rs1 is a negative normal number.   |
  +--------+------------------------------------+
  | 2      | rs1 is a negative subnormal number.|
  +--------+------------------------------------+
  | 3      | rs1 is −0.                         |
  +--------+------------------------------------+
  | 4      | rs1 is +0.                         |
  +--------+------------------------------------+
  | 5      | rs1 is a positive subnormal number.|
  +--------+------------------------------------+
  | 6      | rs1 is a positive normal number.   |
  +--------+------------------------------------+
  | 7      | rs1 is +infinity                   |
  +--------+------------------------------------+
  | 8      | rs1 is a signaling NaN.            |
  +--------+------------------------------------+
  | 9      | rs1 is a quiet NaN.                |
  +--------+------------------------------------+
