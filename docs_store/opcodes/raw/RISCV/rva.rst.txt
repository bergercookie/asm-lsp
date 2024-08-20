RV32A, RV64A Instructions
=========================

lr.w
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |00010|aq|rl|00000|rs1  |010  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | lr.w       rd,rs1

:Description:
  | load a word from the address in rs1, places the sign-extended value in rd, and registers a reservation on the memory address.

:Implementation:
  | x[rd] = LoadReserved32(M[x[rs1]])


sc.w
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |00011|aq|rl|rs2  |rs1  |010  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | sc.w       rd,rs1,rs2

:Description:
  | write a word in rs2 to the address in rs1, provided a valid reservation still exists on that address.
  | SC writes zero to rd on success or a nonzero code on failure.

:Implementation:
  | x[rd] = StoreConditional32(M[x[rs1]], x[rs2])


amoswap.w
----------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |00001|aq|rl|rs2  |rs1  |010  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amoswap.w  rd,rs2,(rs1)

:Description:
  | atomically load a 32-bit signed data value from the address in rs1, place the value into register rd, swap the loaded value and the original 32-bit signed value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO32(M[x[rs1]] SWAP x[rs2])


amoadd.w
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |00000|aq|rl|rs2  |rs1  |010  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amoadd.w   rd,rs2,(rs1)

:Description:
  | atomically load a 32-bit signed data value from the address in rs1, place the value into register rd, apply add the loaded value and the original 32-bit signed value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO32(M[x[rs1]] + x[rs2])




amoxor.w
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |00100|aq|rl|rs2  |rs1  |010  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amoxor.w   rd,rs2,(rs1)

:Description:
  | atomically load a 32-bit signed data value from the address in rs1, place the value into register rd, apply exclusive or the loaded value and the original 32-bit signed value in rs2, then store the result back to the address in rs1.
:Implementation:
  | x[rd] = AMO32(M[x[rs1]] ^ x[rs2])




amoand.w
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |01100|aq|rl|rs2  |rs1  |010  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amoand.w   rd,rs2,(rs1)

:Description:
  | atomically load a 32-bit signed data value from the address in rs1, place the value into register rd, apply and the loaded value and the original 32-bit signed value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO32(M[x[rs1]] & x[rs2])




amoor.w
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |01000|aq|rl|rs2  |rs1  |010  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amoor.w    rd,rs2,(rs1)

:Description:
  | atomically load a 32-bit signed data value from the address in rs1, place the value into register rd, apply or the loaded value and the original 32-bit signed value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO32(M[x[rs1]] | x[rs2])




amomin.w
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |10000|aq|rl|rs2  |rs1  |010  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amomin.w   rd,rs2,(rs1)

:Description:
  | atomically load a 32-bit signed data value from the address in rs1, place the value into register rd, apply min operator the loaded value and the original 32-bit signed value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO32(M[x[rs1]] MIN x[rs2])




amomax.w
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |10100|aq|rl|rs2  |rs1  |010  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amomax.w   rd,rs2,(rs1)

:Description:
  | atomically load a 32-bit signed data value from the address in rs1, place the value into register rd, apply max operator the loaded value and the original 32-bit signed value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO32(M[x[rs1]] MAX x[rs2])




amominu.w
----------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |11000|aq|rl|rs2  |rs1  |010  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amominu.w  rd,rs2,(rs1)

:Description:
  | atomically load a 32-bit unsigned data value from the address in rs1, place the value into register rd, apply unsigned min the loaded value and the original 32-bit unsigned value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO32(M[x[rs1]] MINU x[rs2])




amomaxu.w
----------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |11100|aq|rl|rs2  |rs1  |010  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amomaxu.w  rd,rs2,(rs1)

:Description:
  | atomically load a 32-bit unsigned data value from the address in rs1, place the value into register rd, apply unsigned max the loaded value and the original 32-bit unsigned value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO32(M[x[rs1]] MAXU x[rs2])
