RV64A Instructions
==================

lr.d
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |00010|aq|rl|rs2  |rs1  |011  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | lr.d       rd,rs1

:Description:
  | load a 64-bit data from the address in rs1, places value in rd, and registers a reservation on the memory address.

:Implementation:
  | x[rd] = LoadReserved64(M[x[rs1]])


sc.d
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |00011|aq|rl|rs2  |rs1  |011  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | sc.d       rd,rs1,rs2

:Description:
  | write a 64-bit data in rs2 to the address in rs1, provided a valid reservation still exists on that address.
  | SC writes zero to rd on success or a nonzero code on failure.

:Implementation:
  | x[rd] = StoreConditional64(M[x[rs1]], x[rs2])



amoswap.d
----------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |00001|aq|rl|rs2  |rs1  |011  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amoswap.d  rd,rs2,(rs1)

:Description:
  | atomically load a 64-bit data value from the address in rs1, place the value into register rd, swap the loaded value and the original 64-bit value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO64(M[x[rs1]] SWAP x[rs2])




amoadd.d
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |00000|aq|rl|rs2  |rs1  |011  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amoadd.d   rd,rs2,(rs1)

:Description:
  | atomically load a 64-bit data value from the address in rs1, place the value into register rd, apply add the loaded value and the original 64-bit value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO64(M[x[rs1]] + x[rs2])




amoxor.d
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |00100|aq|rl|rs2  |rs1  |011  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amoxor.d   rd,rs2,(rs1)

:Description:
  | atomically load a 64-bit data value from the address in rs1, place the value into register rd, apply xor the loaded value and the original 64-bit value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO64(M[x[rs1]] ^ x[rs2])




amoand.d
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |01100|aq|rl|rs2  |rs1  |011  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amoand.d   rd,rs2,(rs1)

:Description:
  | atomically load a 64-bit data value from the address in rs1, place the value into register rd, apply and the loaded value and the original 64-bit value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO64(M[x[rs1]] & x[rs2])




amoor.d
--------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |01000|aq|rl|rs2  |rs1  |011  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amoor.d    rd,rs2,(rs1)

:Description:
  | atomically load a 64-bit data value from the address in rs1, place the value into register rd, apply or the loaded value and the original 64-bit value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO64(M[x[rs1]] | x[rs2])




amomin.d
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |10000|aq|rl|rs2  |rs1  |011  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amomin.d   rd,rs2,(rs1)

:Description:
  | atomically load a 64-bit data value from the address in rs1, place the value into register rd, apply min the loaded value and the original 64-bit value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO64(M[x[rs1]] MIN x[rs2])



amomax.d
---------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |10100|aq|rl|rs2  |rs1  |011  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amomax.d   rd,rs2,(rs1)

:Description:
  | atomically load a 64-bit data value from the address in rs1, place the value into register rd, apply max the loaded value and the original 64-bit value in rs2, then store the result back to the address in rs1.
:Implementation:
  | x[rd] = AMO64(M[x[rs1]] MAX x[rs2])




amominu.d
----------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |11000|aq|rl|rs2  |rs1  |011  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amominu.d  rd,rs2,(rs1)

:Description:
  | atomically load a 64-bit data value from the address in rs1, place the value into register rd, apply unsigned min the loaded value and the original 64-bit value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO64(M[x[rs1]] MINU x[rs2])


amomaxu.d
----------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+--+--+-----+-----+-----+-----+-----+---+
  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+--+--+-----+-----+-----+-----+-----+---+
  |11100|aq|rl|rs2  |rs1  |011  |rd   |01011|11 |
  +-----+--+--+-----+-----+-----+-----+-----+---+



:Format:
  | amomaxu.d  rd,rs2,(rs1)

:Description:
  | atomically load a 64-bit data value from the address in rs1, place the value into register rd, apply unsigned max the loaded value and the original 64-bit value in rs2, then store the result back to the address in rs1.

:Implementation:
  | x[rd] = AMO64(M[x[rs1]] MAXU x[rs2])
