RV32M, RV64M Instructions
=========================

mul
----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|01   |rs2  |rs1  |000  |rd   |01100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | mul        rd,rs1,rs2

:Description:
  | performs an XLEN-bit :math:`\times` XLEN-bit multiplication of signed rs1 by signed rs2 and places the lower XLEN bits in the destination register.

:Implementation:
  | x[rd] = x[rs1] × x[rs2]


mulh
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|01   |rs2  |rs1  |001  |rd   |01100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | mulh       rd,rs1,rs2

:Description:
  | performs an XLEN-bit :math:`\times` XLEN-bit multiplication of signed rs1 by signed rs2 and places the upper XLEN bits in the destination register.

:Implementation:
  | x[rd] = (x[rs1] s×s x[rs2]) >>s XLEN


mulhsu
-------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|01   |rs2  |rs1  |010  |rd   |01100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | mulhsu     rd,rs1,rs2

:Description:
  | performs an XLEN-bit :math:`\times` XLEN-bit multiplication of signed rs1 by unsigned rs2 and places the upper XLEN bits in the destination register.

:Implementation:
  | x[rd] = (x[rs1] s :math:`\times` x[rs2]) >>s XLEN


mulhu
------

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|01   |rs2  |rs1  |011  |rd   |01100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | mulhu      rd,rs1,rs2

:Description:
  | performs an XLEN-bit :math:`\times` XLEN-bit multiplication of unsigned rs1 by unsigned rs2 and places the upper XLEN bits in the destination register.

:Implementation:
  | x[rd] = (x[rs1] u :math:`\times` x[rs2]) >>u XLEN


div
----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|01   |rs2  |rs1  |100  |rd   |01100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | div        rd,rs1,rs2

:Description:
  | perform an XLEN bits by XLEN bits signed integer division of rs1 by rs2, rounding towards zero.

:Implementation:
  | x[rd] = x[rs1] /s x[rs2]




divu
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|01   |rs2  |rs1  |101  |rd   |01100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | divu       rd,rs1,rs2

:Description:
  | perform an XLEN bits by XLEN bits unsigned integer division of rs1 by rs2, rounding towards zero.

:Implementation:
  | x[rd] = x[rs1] /u x[rs2]


rem
----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|01   |rs2  |rs1  |110  |rd   |01100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | rem        rd,rs1,rs2

:Description:
  | perform an XLEN bits by XLEN bits signed integer reminder of rs1 by rs2.

:Implementation:
  | x[rd] = x[rs1] %s x[rs2]


remu
-----

.. tabularcolumns:: |c|c|c|c|c|c|c|c|
.. table::

  +-----+-----+-----+-----+-----+-----+-----+---+
  |31-27|26-25|24-20|19-15|14-12|11-7 |6-2  |1-0|
  +-----+-----+-----+-----+-----+-----+-----+---+
  |00000|01   |rs2  |rs1  |111  |rd   |01100|11 |
  +-----+-----+-----+-----+-----+-----+-----+---+



:Format:
  | remu       rd,rs1,rs2

:Description:
  | perform an XLEN bits by XLEN bits unsigned integer reminder of rs1 by rs2.

:Implementation:
  | x[rd] = x[rs1] %u x[rs2]
