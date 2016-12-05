from __future__ import print_function
import pkg_resources
import xml.etree.ElementTree as ET


class Instruction:
    """Instruction is defines by its mnemonic name (in Intel-style assembly).

    An instruction may have multiple forms, that mainly differ by operand types.

    :ivar name: instruction name in Intel-style assembly (PeachPy, NASM and YASM assemblers).
    :ivar summary: a summary description of the instruction name.
    :ivar forms: a list of :class:`InstructionForm` objects representing the instruction forms.
    """
    def __init__(self, name):
        self.name = name
        self.summary = None
        self.forms = []

    def __str__(self):
        """Returns string representation of the instruction and the number of instruction forms"""
        return "%s (%d forms)" % (self.name, len(self.forms))

    def __repr__(self):
        return str(self)


class InstructionForm:
    """Instruction form is a combination of mnemonic name and operand types.

    An instruction form may have multiple possible encodings.

    :ivar name: instruction name in PeachPy assembler.
    :ivar gas_name: instruction form name in GNU assembler (gas).
    :ivar cancelling_inputs: indicates that the instruction form has not dependency on the values of input operands
        when they refer to the same register. E.g. **VPXORD zmm1, zmm0, zmm0** does not depend on *zmm0*.

        Instruction forms with cancelling inputs have only two input operands, which have the same register type.

    :ivar operands: a list of :class:`Operand` objects representing the instruction operands.
    :ivar implicit_inputs: a set of register names that are implicitly read by this instruction.
    :ivar implicit_outputs: a set of register names that are implicitly written by this instruction.
    :ivar encodings: a list of :class:`Encoding` objects representing the possible encodings for this instruction.
    """

    def __init__(self, name):
        self.name = name
        self.gas_name = None
        self.cancelling_inputs = None
        self.operands = []
        self.implicit_inputs = set()
        self.implicit_outputs = set()
        self.encodings = []

    def __str__(self):
        """Returns string representation of the instruction form and its operands in Intel-style assembly"""
        if self.operands:
            return self.name + " " + ", ".join([operand.type for operand in self.operands])
        else:
            return self.name

    def __repr__(self):
        return str(self)


class Operand:
    """An explicit instruction operand.

    :ivar type: the type of the instruction operand. Possible values are:

        "1"
            The constant value `1`.

        "3"
            The constant value `3`.

        "al"
            The al register.

        "ax"
            The ax register.

        "eax"
            The eax register.

        "rax"
            The rax register.

        "cl"
            The cl register.

        "rel8"
            An 8-bit signed offset relative to the address of instruction end.

        "rel32"
            A 32-bit signed offset relative to the address of instruction end.

        "imm8"
            An 8-bit immediate value.

        "imm16"
            A 16-bit immediate value.

        "imm32"
            A 32-bit immediate value.

        "imm64"
            A 64-bit immediate value.

        "r8"
            An 8-bit general-purpose register (al, bl, cl, dl, sil, dil, bpl, spl, r8b-r15b).

        "r16"
            A 16-bit general-purpose register (ax, bx, cx, dx, si, di, bp, sp, r8w-r15w).

        "r32"
            A 32-bit general-purpose register (eax, ebx, ecx, edx, esi, edi, ebp, esp, r8d-r15d).

        "r64"
            A 64-bit general-purpose register (rax, rbx, rcx, rdx, rsi, rdi, rbp, rsp, r8-r15).

        "zmm"
            A 512-bit ZMM SIMD register (zmm0-zmm31).

        "zmm{k}"
            A 512-bit ZMM SIMD register (zmm0-zmm31), optionally masked by a mask register (k1-k7).

        "S(zmm)"
            A 512-bit ZMM SIMD register (zmm0-zmm31), optionally with an elements swizzle ({cdab}, {badc}, \
            {dacb}, {aaaa}, {bbbb}, {cccc}, {dddd}).

        "Cf32(zmm)"
            A 512-bit ZMM SIMD register (zmm0-zmm31), optionally with a single-precision store down-conversion \
            ({float16}, {uint8}, {sint8}, {uint16}, {sint16}).

        "Ci32(zmm)"
            A 512-bit ZMM SIMD register (zmm0-zmm31), optionally with a 32-bit integer store down-conversion \
            ({uint8}, {sint8}, {uint16}, {sint16}).

        "k"
            A mask register (k0-k7).

        "k{k}"
            A mask register (k0-k7), optionally masked by another mask register (k1-k7).

        "m"
            A memory operand of any size.

        "m8"
            An 8-bit memory operand.

        "m16"
            A 16-bit memory operand.

        "m32"
            A 32-bit memory operand.

        "m64"
            A 64-bit memory operand.

        "m80"
            An 80-bit memory operand.

        "m128"
            A 128-bit memory operand.

        "m512"
            A 512-bit memory operand.

        "m512{k}"
            A 512-bit memory operand, optionally masked by a mask register (k1-k7).

        "BCf32(m512)"
            A 512-bit memory operand, optionally with a single-precision memory broadcast/conversion ({1to16}, \
            {4to16}, {float16}, {uint8}, {uint16}, {sint16})

        "BCi32(m512)"
            A 512-bit memory operand, optionally with a 32-bit integer memory broadcast/conversion ({1to16}, {4to16}, \
            {uint8}, {sint8}, {uint16}, {sint16})

        "B64(m512)"
            A 512-bit memory operand, optionally with a 64-bit elements broadcast ({1to8}, {4to8})

        "Cf32(m512)"
            A 512-bit memory operand, optionally with a single-precision memory up-conversion ({float16}, \
            {uint8}, {sint8}, {uint16}, {sint16})

        "Ci32(m512)"
            A 512-bit memory operand, optionally with a 32-bit integer memory up-conversion ({uint8}, {sint8}, \
            {uint16}, {sint16})

        "vm32z"
            A vector of memory addresses using VSIB with 32-bit indices in ZMM register.

        "vm32z{k}"
            A vector of memory addresses using VSIB with 32-bit indices in ZMM register masked by a mask register \
            (k1-k7).

        "Cf32(vm32z)"
            A vector of memory addresses using VSIB with 32-bit indices in ZMM register, optionally with a \
            single-precision memory up-conversion ({float16}, {uint8}, {sint8}, {uint16}, {sint16}).

        "Ci32(vm32z)"
            A vector of memory addresses using VSIB with 32-bit indices in ZMM register, optionally with a 32-bit \
            integer memory up-conversion ({uint8}, {sint8}, {uint16}, {sint16}).

        "Cf32(vm32z){k}"
            A vector of memory addresses using VSIB with 32-bit indices in ZMM register masked by a mask register, \
            optionally with a single-precision memory up-conversion ({float16}, {uint8}, {sint8}, {uint16}, {sint16}).

        "{sae}"
            Suppress-all-exceptions modifier. This operand is optional and can be omitted.

        "{er}"
            Embedded rounding control. This operand is optional and can be omitted.

    :ivar allow_conversion: for a memory operand with BCf32/BCi32 primitive indicates if memory conversion primitive \
    can be used for the operand. For all other types of operands this variable is meaningless, and its value is None.
    :ivar allow_1to16: for a memory operand with BCf32/BCi32 primitive indicates if {1to16} primitive can be used for \
    the operand. For all other types of operands this variable is meaningless, and its value is None.

    :ivar is_input: indicates if the instruction reads the variable specified by this operand.
    :ivar is_output: indicates if the instruction writes the variable specified by this operand.
    :ivar extended_size: for immediate operands the size of the value in bytes after size-extension.

        The extended size affects which operand values can be encoded. E.g. a signed imm8 operand \
        would normally values in the [-128, 127] range. But if it is extended to 4 bytes, it can also \
        encode values in [2**32 - 128, 2**32 - 1] range.
    """
    def __init__(self, type):
        self.type = type
        self.allow_1to16 = None
        self.allow_conversion = None
        self.is_input = False
        self.is_output = False
        self.extended_size = None

        if self.type.startswith("S") and self.is_memory:
            self.allow_1to16 = True
            self.allow_conversion = True

    def __str__(self):
        """Return string representation of the operand type and its read/write attributes"""
        return {
            (False, False): self.type,
            (True, False): "[in] " + self.type,
            (False, True): "[out] " + self.type,
            (True, True): "[in/out] " + self.type
        }[(self.is_input, self.is_output)]

    def __repr__(self):
        return str(self)

    @property
    def is_variable(self):
        """Indicates whether this operand refers to a variable (i.e. specifies either a register or a memory location)"""
        return self.is_input or self.is_output

    @property
    def is_register(self):
        """Indicates whether this operand specifies a register"""
        return self.type \
            in ["al", "cl", "ax", "eax", "rax", "r8", "r16", "r32", "r64", "r8l", "r16l", "r32l",
                "zmm", "zmm{k}", "S(zmm)", "Cf32(zmm)", "Ci32(zmm)", "k", "k{k}"]

    @property
    def is_memory(self):
        """Indicates whether this operand specifies a memory location"""
        return self.type \
            in ["m", "m8", "m16", "m32", "m64", "m80", "m128",
                "m512", "m512{k}", "BCf32(m512)", "BCi32(m512)", "B64(m512)", "Cf32(m512)", "Ci32(m512)",
                "vm32z", "vm32z{k}", "Cf32(vm32z)", "Ci32(vm32z)", "Cf32(vm32z){k}"]

    @property
    def is_immediate(self):
        """Indicates whether this operand is an immediate constant"""
        return self.type in ["imm8", "imm16", "imm32", "imm64"]


class Encoding:
    """Instruction encoding

    :ivar components: a list of :class:`Prefix`, :class:`REX`, :class:`VEX`, :class:`Opcode`, :class:`ModRM`, \
        :class:`RegisterByte`, :class:`Immediate`, :class:`DataOffset`, :class:`CodeOffset` objects that \
        specify the components of encoded instruction
    """

    def __init__(self):
        self.components = []


class Prefix:
    """0x66/0xF2/0xF3 prefix

    :ivar is_mandatory: indicates that the prefix is used not for its primary purpose, but for extending instruction \
        opcode. Mandatory prefixes are common in SSE instructions. Non-mandatory prefix is usually 0x66 that modifies \
        the instruction to operate on 16-bit operands.
    :ivar byte: numerical representation of the prefix byte.
    """

    def __init__(self):
        self.byte = None
        self.is_mandatory = None


class REX:
    """REX prefix.

    Encoding may have only one REX prefix and if present, it immediately precedes the opcode.

    :ivar is_mandatory: indicates whether the REX prefix must be encoded even if no extended registers are used.

        REX is mandatory for most 64-bit instructions (encoded with REX.W = 1) and instructions that operate on the \
        extended set of 8-bit registers (to indicate access to dil/sil/bpl/spl as opposed to ah/bh/ch/dh which use the \
        same ModR/M).

    :ivar W: the REX.W bit. Possible values are 0, 1, and None.

        None indicates that the bit is ignored.

    :ivar R: the REX.R bit. Possible values are 0, 1, None, or a reference to one of the instruction operands.

        The value None indicates that this bit is ignored. \
        If R is a reference to an instruction operand, the operand is of register type and REX.R bit specifies the \
        high bit (bit 3) of the register number.

    :ivar B: the REX.B bit. Possible values are 0, 1, None, or a reference to one of the instruction operands.

        The value None indicates that this bit is ignored. \
        If R is a reference to an instruction operand, the operand can be of register or memory type. If the operand \
        is of register type, the REX.R bit specifies the high bit (bit 3) of the register number, and the REX.X bit is \
        ignored. If the operand is of memory type, the REX.R bit specifies the high bit (bit 3) of the base register \
        number, and the X instance variable refers to the same operand.

    :ivar X: the REX.X bit. Possible values are 0, 1, None, or a reference to one of the instruction operands.

        The value None indicates that this bit is ignored. \
        If X is a reference to an instruction operand, the operand is of memory type and the REX.X bit specifies the \
        high bit (bit 3) of the index register number, and the B instance variable refers to the same operand.
    """

    def __init__(self):
        self.is_mandatory = None
        self.W = None
        self.R = None
        self.X = None
        self.B = None

    def set_ignored(self, w=0, r=0, x=0, b=0):
        """Sets values for ignored bits

        :param int w: the value (0 or 1) to be assigned to REX.W bit if it is ignored.
        :param int r: the value (0 or 1) to be assigned to REX.R bit if it is ignored.
        :param int x: the value (0 or 1) to be assigned to REX.X bit if it is ignored.
        :param int b: the value (0 or 1) to be assigned to REX.B bit if it is ignored.
        """
        assert w in [0, 1], "REX.W can be only 0 or 1"
        assert r in [0, 1], "REX.R can be only 0 or 1"
        assert x in [0, 1], "REX.X can be only 0 or 1"
        assert b in [0, 1], "REX.B can be only 0 or 1"
        if self.W is None:
            self.W = w
        if self.R is None:
            self.R = r
        if self.X is None:
            self.X = x
        if self.B is None:
            self.B = b


class VEX:
    """VEX prefix.

    Encoding may have only one VEX prefix and if present, it immediately precedes the opcode, and no other prefix is \
    allowed.

    :ivar mmmmm: the VEX m-mmmm (implied leading opcode bytes) field. Possible values are:

        0b00001
            Implies 0x0F leading opcode byte.

        0b00011
            Implies 0x0F 0x3A leading opcode bytes.

        Only VEX prefix with m-mmmm equal to 0b00001 could be encoded in two bytes.

    :ivar pp: the VEX pp (implied legacy prefix) field. Possible values are:

        0b00
            No implied prefix.

        0b01
            Implied 0x66 prefix.

        0b10
            Implied 0xF3 prefix.

        0b11
            Implied 0xF2 prefix.

    :ivar W: the VEX.W bit. Possible values are 0, 1, and None.

        None indicates that the bit is ignored.

    :ivar L: the VEX.L bit. Possible values are 0 or 1.

    :ivar R: the VEX.R bit. Possible values are 0, 1, None, or a reference to one of the instruction operands.

        The value None indicates that this bit is ignored. \
        If R is a reference to an instruction operand, the operand is of register type and VEX.R bit specifies the \
        high bit (bit 3) of the register number.

    :ivar B: the VEX.B bit. Possible values are 0, 1, None, or a reference to one of the instruction operands.

        The value None indicates that this bit is ignored. \
        If R is a reference to an instruction operand, the operand can be of register or memory type. If the operand is \
        of register type, the VEX.R bit specifies the high bit (bit 3) of the register number, and the VEX.X bit is \
        ignored. If the operand is of memory type, the VEX.R bit specifies the high bit (bit 3) of the base register \
        number, and the X instance variable refers to the same operand.

    :ivar X: the VEX.X bit. Possible values are 0, 1, None, or a reference to one of the instruction operands.

        The value None indicates that this bit is ignored. \
        If X is a reference to an instruction operand, the operand is of memory type and the VEX.X bit specifies the \
        high bit (bit 3) of the index register number, and the B instance variable refers to the same operand.

    :ivar vvvv: the VEX vvvv field. Possible values are 0b0000 or a reference to one of the instruction operands.

        The value 0b0000 indicates that this field is not used. \
        If vvvv is a reference to an instruction operand, the operand is of register type and VEX.vvvv field specifies \
        its number.
    """

    def __init__(self):
        self.type = None
        self.mmmmm = None
        self.pp = None
        self.W = None
        self.L = None
        self.R = None
        self.X = None
        self.B = None
        self.vvvv = None

    def set_ignored(self, w=0, r=0, x=0, b=0):
        """Sets values for ignored bits

        :param int w: the value (0 or 1) to be assigned to VEX.W bit if it is ignored.
        :param int r: the value (0 or 1) to be assigned to VEX.R bit if it is ignored.
        :param int x: the value (0 or 1) to be assigned to VEX.X bit if it is ignored.
        :param int b: the value (0 or 1) to be assigned to VEX.B bit if it is ignored.
        """
        assert w in [0, 1], "VEX.W can be only 0 or 1"
        assert r in [0, 1], "VEX.R can be only 0 or 1"
        assert x in [0, 1], "VEX.X can be only 0 or 1"
        assert b in [0, 1], "VEX.B can be only 0 or 1"
        if self.W is None:
            self.W = w
        if self.R is None:
            self.R = r
        if self.X is None:
            self.X = x
        if self.B is None:
            self.B = b


class MVEX:
    """MVEX prefix.

    Encoding may have only one MVEX prefix and if present, it immediately precedes the opcode, and no other prefix is \
    allowed.

    :ivar mmmm: the MVEX mmmm (compressed legacy escape) field. Identical to two low bits of VEX.m-mmmm field. \
    Possible values are:

        0b0001
            Implies 0x0F leading opcode byte.

        0b0010
            Implies 0x0F 0x38 leading opcode bytes.

        0b0011
            Implies 0x0F 0x3A leading opcode bytes.

    :ivar pp: the MVEX pp (compressed legacy prefix) field. Possible values are:

        0b00
            No implied prefix.

        0b01
            Implied 0x66 prefix.

        0b10
            Implied 0xF3 prefix.

        0b11
            Implied 0xF2 prefix.

    :ivar W: the MVEX.W bit. Possible values are 0, 1, and None.

        None indicates that the bit is ignored.

    :ivar RR: the MVEX.R'R bits. Possible values are None, or a reference to an register-type instruction operand.

        None indicates that the field is ignored. \
        The R' bit specifies bit 4 of the register number and the R bit specifies bit 3 of the register number.

    :ivar B: the MVEX.B bit. Possible values are None, or a reference to one of the instruction operands.

        None indicates that this bit is ignored. \
        If R is a reference to an instruction operand, the operand can be of register or memory type. If the operand is\
        of register type, the MVEX.R bit specifies the high bit (bit 3) of the register number, and the MVEX.X bit is \
        ignored. If the operand is of memory type, the MVEX.R bit specifies the high bit (bit 3) of the base register \
        number, and the X instance variable refers to the same operand.

    :ivar X: the MVEX.X bit. Possible values are None, or a reference to one of the instruction operands.

        The value None indicates that this bit is ignored. \
        If X is a reference to an instruction operand, the operand is of memory type and the MVEX.X bit specifies the \
        high bit (bit 3) of the index register number, and the B instance variable refers to the same operand.

    :ivar vvvv: the MVEX vvvv field. Possible values are 0b0000 or a reference to one of the instruction operands.

        The value 0b0000 indicates that this field is not used. \
        If vvvv is a reference to an instruction operand, the operand is of register type and MVEX.vvvv field \
        specifies the register number.

    :ivar V: the MVEX V field. Possible values are 0, or a reference to one of the instruction operands.

        The value 0 indicates that this field is not used (MVEX.vvvv is not used or encodes a general-purpose register).

    :ivar SSS: the MVEX SSS (swizzle/broadcast/up-convert/down-convert) field. Possible values are 0, or a reference \
    to one of the instruction operands.

        The value 0 indicates that this field is not used. \
        If SSS is a reference to an instruction operand, the operand type either includes a swizzle, broadcast, or \
        conversion primitive, or the operand type is {er} (static rounding control), or the operand type is {sae}. \
        If SSS is a reference to a memory/register operand, it encodes the primitive applied to the operand. \
        If SSS is a reference to a static rounding control operand, it the high bit of MVEX.SSS encodes \
        suppress-all-exceptions mode (1 = enabled, 0 = disabled) and the two low bits encode rounding mode (round \
        to nearest even = 0b00, round down = 0b01, round up = 0b10, round toward zero = 0b11) \
        If SSS is a reference to a suppress-all-exceptions operand, the high bit of MVEX.SSS encodes the \
        suppress-all-exceptions mode (1 = enabled, 0 = disabled) and the two low bits are ignored.

    :ivar aaa: the MVEX aaa (embedded opmask register specifier) field. Possible values are 0 or a reference to one of \
    the instruction operands.

        The value 0 indicates that this field is not used. \
        If aaa is a reference to an instruction operand, the operand supports register mask, and MVEX.aaa encodes the \
        mask register.

    :ivar E: the MVEX E (eviction hint/MVEX.SSS override) bit. Possible values are 0, 1, or a reference to an \
    instruction operand.

        The value 0 indicates that MVEX.SSS field specifies swizzle primitive for a register operand. \
        The value 1 indicates that MVEX.SSS field specifies static rounding mode and/or suppress-all-exceptions mode \
        for the instruction. \
        If E is a reference to an instruction operand, the operand is of memory type, and MVEX.E encodes whether \
        eviction hint applies to the operand (1 = eviction hint set, 0 = eviction hint not set).

    :ivar disp8xN: the N value used for encoding compressed 8-bit displacement of memory operands when no broadcast or \
    conversion is specified. Possible values are powers of 2 in [4, 64] range or None.

        None indicates that this instruction form does not use displacement (the form has no memory operands).

        When broadcast or conversion is specified, N is decreased by the following factors:

            {1to16}
                N is decreased by 16

            {4to16}
                N is decreased by 4

            {1to8}
                N is decreased by 8

            {4to8}
                N is decreased by 2

            {float16}
                N is decreased by 2

            {uint16}
                N is decreased by 2

            {sint16}
                N is decreased by 2

            {uint8}
                N is decreased by 4

            {sint8}
                N is decreased by 4
    """

    def __init__(self):
        self.mmmm = None
        self.pp = None
        self.W = None
        self.LL = None
        self.RR = None
        self.B = None
        self.X = None
        self.vvvv = None
        self.V = None
        self.SSS = None
        self.aaa = None

    def set_ignored(self, w=0, rr=0, x=0, z=0):
        """Sets values for ignored bits

        :param int w: the value (0 or 1) to be assigned to MVEX.W bit if it is ignored.
        :param int rr: the value (0b00, 0b01, 0b10, or 0b11) to be assigned to MVEX.R'R field if it is ignored.
        :param int x: the value (0 or 1) to be assigned to MVEX.X bit if it is ignored.
        :param int z: the value (0 or 1) to be assigned to MVEX.z bit if it is ignored.
        """
        assert w in [0, 1], "MVEX.W can be only 0 or 1"
        assert rr in [0b00, 0b01, 0b10, 0b11], "MVEX.R'R must be a 2-bit integer value"
        assert x in [0, 1], "MVEX.X can be only 0 or 1"
        assert z in [0, 1], "MVEX.z can be only 0 or 1"
        if self.W is None:
            self.W = w
        if self.RR is None:
            self.RR = rr
        if self.X is None:
            self.X = x
        if self.z is None:
            self.z = z


class Opcode:
    """Operation code

    Encoding may include more than one opcode. Opcodes do not necessarily go in sequence.

    :ivar byte: operation code as a byte integer (0 <= `byte` <= 255)
    :ivar addend: None or a reference to an instruction operand.

        If addend is a reference to an instruction operand, the operand is of register type and the three lowest bits \
        of its number must be ORed with `byte` to produce the final opcode value.
    """
    def __init__(self, byte):
        self.byte = byte
        self.addend = None

    def __str__(self):
        return "0x%02X".format(self.byte)

    def __repr__(self):
        return str(self)


class ModRM:
    """Mod R/M byte that can encode a register operand, a memory operand, or provide an opcode extension.

    If memory operand requires SIB byte, the SIB byte immediately follows the Mod R/M byte in instruction encoding.

    :ivar mode: addressing mode. Possible values are 0b11 or a reference to an instruction operand.

        If mode value is 0b11, the Mod R/M encodes two register operands or a register operand and an opcode extension.

        If mode is a reference to an instruction operand, the operand has memory type and its addressing mode must be \
        coded instruction the Mod R/M mode field.

    :ivar rm: a register or memory operand. Must be a reference to an instruction operand.

        If rm is a reference to a operand, rm specifies bits 0-2 of the register number. If the operand is of memory \
        type, rm specifies bits 0-2 of the base register number unless a SIB byte is used.

    :ivar reg: a register or an opcode extension. Possible values are an int value, or a reference to an instruction \
        operand.

        If reg is an int value, this value extends the opcode and must be directly coded in the reg field.

        If reg is a reference to an instruction operand, the operand is of register type, and the reg field specifies \
        bits 0-2 of the register number.
    """
    def __init__(self):
        self.mode = None
        self.rm = None
        self.reg = None

    def set_ignored(self, mode=0b11, rm=0):
        """Sets values for ignored fields

        :param int mode: the value (0b00, 0b01, 0b10, or 0b11) to be assigned to Mod R/M mode field if it is ignored.
        :param int rm: the value (an integer, 0 <= rm <= 7) to be assigned to Mod R/M rm field if it is ignored.
        """
        assert mode in [0b00, 0b01, 0b10, 0b11], "Mod R/M mode must be 0b00, 0b01, 0b10 or 0b11"
        assert rm in [0, 1, 2, 3, 4, 5, 6, 7], "Mod R/M rm must be an integer in 0-7 range"
        if self.mode is None:
            self.mode = mode
        if self.rm is None:
            self.rm = rm


class Immediate:
    """Immediate constant embedded into instruction encoding.

    :ivar size: size of the constant in bytes. Possible values are 1, 2, 4, or 8.
    :ivar value: value of the constant. Can be an int value or a reference to an instruction operand.

        If value is a reference to an instruction operand, the operand has "imm" type of the matching size.
    """
    def __init__(self):
        self.size = None
        self.value = None


class RegisterByte:
    def __init__(self):
        self.register = None
        self.payload = None


class CodeOffset:
    """Relative code offset embedded into instruction encoding.

    Offset is relative to the end of the instruction.

    :ivar size: size of the offset in bytes. Possible values are 1 or 4.
    :ivar value: value of the offset. Must be a reference to an instruction operand.

        The instruction operand has "rel" type of the matching size.
    """
    def __init__(self):
        self.size = None
        self.value = None


class DataOffset:
    """Absolute data offset embedded into instruction encoding.

    Only MOV instruction has forms that use direct data offset.

    :ivar size: size of the offset in bytes. Possible values are 4 or 8.
    :ivar value: value of the offset. Must be a reference to an instruction operand.

        The instruction operand has "moffs" type of the matching size.
    """
    def __init__(self):
        self.size = None
        self.value = None


def _parse_boolean(xml_boolean):
    """Converts strings "true" and "false" from XML files to Python bool"""

    if xml_boolean is not None:
        assert xml_boolean in ["true", "false"], \
            "The boolean string must be \"true\" or \"false\""
        return {"true": True, "false": False}[xml_boolean]


def _parse_value(value, operands, base=None):
    """Parses string, which can be a number, a reference to an operand, or None

    :param value: the string to parse.
    :param operands: the list of operands. If value starts with "#", it is interpreted as a reference to an operand.
    :param base: the base of the integer representation of value.
    """
    if value is not None:
        if value.startswith("#"):
            return operands[int(value[1:])]
        else:
            assert base is not None
            return int(value, base)


def read_instruction_set(filename=None):
    """Reads instruction set data from an XML file and returns a list of :class:`Instruction` objects

    :param filename: path to an XML file with instruction set data
    """
    if filename is None:
        data = pkg_resources.resource_stream("opcodes", "k1om.xml")
        xml_tree = ET.parse(data)
    else:
        xml_tree = ET.parse(filename)

    xml_instruction_set = xml_tree.getroot()
    assert xml_instruction_set.tag == "InstructionSet"
    assert xml_instruction_set.attrib["name"] == "k1om"

    instruction_set = []
    for xml_instruction in xml_instruction_set:
        assert xml_instruction.tag == "Instruction"
        instruction = Instruction(xml_instruction.attrib["name"])
        instruction.summary = xml_instruction.attrib["summary"]
        for xml_instruction_form in xml_instruction:
            instruction_form = InstructionForm(instruction.name)
            instruction_form.gas_name = xml_instruction_form.attrib["gas-name"]
            instruction_form.cancelling_inputs = \
                _parse_boolean(xml_instruction_form.attrib.get("cancelling-inputs", "false"))
            for xml_operand in xml_instruction_form.findall("Operand"):
                operand = Operand(xml_operand.attrib["type"])
                allow_conversion = _parse_boolean(xml_operand.attrib.get("allow-conversion"))
                if allow_conversion is not None:
                    operand.allow_conversion = allow_conversion
                allow_1to16 = _parse_boolean(xml_operand.attrib.get("allow-1to16"))
                if allow_1to16 is not None:
                    operand.allow_1to16 = allow_1to16
                operand.is_input = _parse_boolean(xml_operand.attrib.get("input", "false"))
                operand.is_output = _parse_boolean(xml_operand.attrib.get("output", "false"))
                operand.extended_size = _parse_value(xml_operand.attrib.get("extended-size"), [], 10)
                instruction_form.operands.append(operand)
            for xml_implicit_operand in xml_instruction_form.findall("ImplicitOperand"):
                if _parse_boolean(xml_implicit_operand.attrib["input"]):
                    instruction_form.implicit_inputs.add(xml_implicit_operand.attrib["id"])
                if _parse_boolean(xml_implicit_operand.attrib["output"]):
                    instruction_form.implicit_outputs.add(xml_implicit_operand.attrib["id"])
            for xml_encoding in xml_instruction_form.findall("Encoding"):
                encoding = Encoding()
                for xml_component in xml_encoding:
                    if xml_component.tag == "Prefix":
                        prefix = Prefix()
                        prefix.byte = _parse_value(xml_component.attrib.get("byte"), [], 16)
                        assert prefix.byte in [0x66, 0xF2, 0xF3]
                        prefix.is_mandatory = _parse_boolean(xml_component.attrib.get("mandatory"))
                        encoding.components.append(prefix)
                    elif xml_component.tag == "REX":
                        rex = REX()
                        rex.is_mandatory = _parse_boolean(xml_component.attrib["mandatory"])
                        rex.W = _parse_value(xml_component.attrib.get("W"), [], 2)
                        rex.R = _parse_value(xml_component.attrib.get("R"), instruction_form.operands, 2)
                        rex.B = _parse_value(xml_component.attrib.get("B"), instruction_form.operands, 2)
                        rex.X = _parse_value(xml_component.attrib.get("X"), instruction_form.operands, 2)
                        encoding.components.append(rex)
                    elif xml_component.tag == "VEX":
                        vex = VEX()
                        vex.pp = int(xml_component.attrib["pp"], 2)
                        vex.mmmmm = int(xml_component.attrib["m-mmmm"], 2)
                        vex.W = _parse_value(xml_component.attrib.get("W"), [], 2)
                        vex.L = int(xml_component.attrib["L"], 2)
                        vex.R = _parse_value(xml_component.attrib.get("R"), instruction_form.operands, 2)
                        vex.B = _parse_value(xml_component.attrib.get("B"), instruction_form.operands, 2)
                        vex.X = _parse_value(xml_component.attrib.get("X"), instruction_form.operands)
                        vex.vvvv = _parse_value(xml_component.attrib.get("vvvv"), instruction_form.operands, 2)
                        encoding.components.append(vex)
                    elif xml_component.tag == "MVEX":
                        mvex = MVEX()
                        mvex.pp = int(xml_component.attrib["pp"], 2)
                        mvex.mmmm = int(xml_component.attrib["mmmm"], 2)
                        mvex.W = _parse_value(xml_component.attrib.get("W"), [], 2)
                        mvex.vvvv = _parse_value(xml_component.attrib["vvvv"], instruction_form.operands, 2)
                        mvex.V = _parse_value(xml_component.attrib["V"], instruction_form.operands, 2)
                        mvex.SSS = _parse_value(xml_component.attrib["SSS"], instruction_form.operands, 2)
                        mvex.aaa = _parse_value(xml_component.attrib["aaa"], instruction_form.operands, 2)
                        mvex.X = _parse_value(xml_component.attrib.get("X"), instruction_form.operands)
                        mvex.B = _parse_value(xml_component.attrib["B"], instruction_form.operands)
                        mvex.RR = _parse_value(xml_component.attrib.get("RR"), instruction_form.operands)
                        encoding.components.append(mvex)
                    elif xml_component.tag == "Opcode":
                        opcode = Opcode(int(xml_component.attrib["byte"], 16))
                        opcode.addend = _parse_value(xml_component.get("addend"), instruction_form.operands)
                        encoding.components.append(opcode)
                    elif xml_component.tag == "ModRM":
                        modrm = ModRM()
                        modrm.mode = _parse_value(xml_component.attrib.get("mode"), instruction_form.operands, 2)
                        modrm.reg = _parse_value(xml_component.attrib.get("reg"), instruction_form.operands, 10)
                        modrm.rm = _parse_value(xml_component.attrib.get("rm"), instruction_form.operands)
                        encoding.components.append(modrm)
                    elif xml_component.tag == "Immediate":
                        assert "size" in xml_component.attrib
                        immediate = Immediate()
                        immediate.size = int(xml_component.attrib["size"])
                        assert immediate.size in [1, 2, 4, 8]
                        immediate.value = _parse_value(xml_component.attrib.get("value"), instruction_form.operands)
                        encoding.components.append(immediate)
                    elif xml_component.tag == "CodeOffset":
                        assert "size" in xml_component.attrib
                        code_offset = CodeOffset()
                        code_offset.size = int(xml_component.attrib["size"])
                        assert code_offset.size in [1, 4]
                        code_offset.value = _parse_value(xml_component.attrib.get("value"), instruction_form.operands)
                        encoding.components.append(code_offset)
                    elif xml_component.tag == "DataOffset":
                        assert "size" in xml_component.attrib
                        data_offset = DataOffset()
                        data_offset.size = int(xml_component.attrib["size"])
                        assert data_offset.size in [4, 8]
                        data_offset.value = _parse_value(xml_component.attrib.get("value"), instruction_form.operands)
                        encoding.components.append(data_offset)
                    else:
                        print("Unknown encoding tag: " + xml_component.tag)

                instruction_form.encodings.append(encoding)
            instruction.forms.append(instruction_form)
        instruction_set.append(instruction)
    return instruction_set
