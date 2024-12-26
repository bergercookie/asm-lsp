RV64M Instructions
==================

mulw
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|01   |rs2  |rs1  |000  |rd   |01110|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | mulw       rd,rs1,rs2

:Description:
  |
:Implementation:
  | x[rd] = sext((x[rs1] × x[rs2])[31:0])


divw
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|01   |rs2  |rs1  |100  |rd   |01110|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | divw       rd,rs1,rs2

:Description:
  | perform an 32 bits by 32 bits signed integer division of rs1 by rs2.

:Implementation:
  | x[rd] = sext(x[rs1][31:0] /s x[rs2][31:0]


divuw
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|01   |rs2  |rs1  |101  |rd   |01110|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | divuw      rd,rs1,rs2

:Description:
  | perform an 32 bits by 32 bits unsigned integer division of rs1 by rs2.

:Implementation:
  | x[rd] = sext(x[rs1][31:0] /u x[rs2][31:0])


remw
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|01   |rs2  |rs1  |110  |rd   |01110|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | remw       rd,rs1,rs2

:Description:
  | perform an 32 bits by 32 bits signed integer reminder of rs1 by rs2.

:Implementation:
  | x[rd] = sext(x[rs1][31:0] %s x[rs2][31:0])


remuw
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|01   |rs2  |rs1  |111  |rd   |01110|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | remuw      rd,rs1,rs2

:Description:
  | perform an 32 bits by 32 bits unsigned integer reminder of rs1 by rs2.

:Implementation:
  | x[rd] = sext(x[rs1][31:0] %u x[rs2][31:0])
