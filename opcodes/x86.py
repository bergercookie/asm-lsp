from __future__ import print_function
import xml.etree.ElementTree as ET
import os


class Instruction:
    """Instruction is defines by its mnemonic name (in Intel-style assembly).

    An instruction may have multiple forms, that mainly differ by operand types.

    :ivar name: instruction name in Intel-style assembly (Peach-Py, NASM and YASM assemblers).
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

    :ivar name: instruction name in Peach-Py, NASM and YASM assemblers.
    :ivar gas_name: instruction form name in GNU assembler (gas).
    :ivar go_name: instruction form name in Go/Plan 9 assembler (8a).

        None means instruction is not supported in Go/Plan 9 assembler.

    :ivar mmx_mode: MMX technology state required or forced by this instruction. Possible values are:

        "FPU"
            Instruction requires the MMX technology state to be clear.

        "MMX"
            Instruction causes transition to MMX technology state.

        None
            Instruction neither affects nor cares about the MMX technology state.

    :ivar xmm_mode: XMM registers state accessed by this instruction. Possible values are:

        "SSE"
            Instruction accesses XMM registers in legacy SSE mode.

        "AVX"
            Instruction accesses XMM registers in AVX mode.

        None
            Instruction does not affect XMM registers and does not change XMM registers access mode.

    :ivar cancelling_inputs: indicates that the instruction form has not dependency on the values of input operands
        when they refer to the same register. E.g. **VPXOR xmm1, xmm0, xmm0** does not depend on *xmm0*.

        Instruction forms with cancelling inputs have only two input operands, which have the same register type.

    :ivar operands: a list of :class:`Operand` objects representing the instruction operands.
    :ivar implicit_inputs: a set of register names that are implicitly read by this instruction.
    :ivar implicit_outputs: a set of register names that are implicitly written by this instruction.
    :ivar isa_extensions: a list of :class:`ISAExtension` objects that represent the ISA extensions required to execute
        the instruction.
    :ivar encodings: a list of :class:`Encoding` objects representing the possible encodings for this instruction.
    """

    def __init__(self, name):
        self.name = name
        self.gas_name = None
        self.go_name = None
        self.mmx_mode = None
        self.xmm_mode = None
        self.cancelling_inputs = None
        self.operands = []
        self.implicit_inputs = set()
        self.implicit_outputs = set()
        self.isa_extensions = []
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

        "cl"
            The cl register.

        "xmm0"
            The xmm0 register.

        "rel8"
            An 8-bit signed offset relative to the address of instruction end.

        "rel32"
            A 32-bit signed offset relative to the address of instruction end.

        "imm4"
            A 4-bit immediate value.

        "imm8"
            An 8-bit immediate value.

        "imm16"
            A 16-bit immediate value.

        "imm32"
            A 32-bit immediate value.

        "r8"
            An 8-bit general-purpose register (al, ah, bl, bh, cl, ch, dl, dh).

        "r16"
            A 16-bit general-purpose register (ax, bx, cx, dx, si, di, bp, sp).

        "r32"
            A 32-bit general-purpose register (eax, ebx, ecx, edx, esi, edi, ebp, esp).

        "mm"
            A 64-bit MMX SIMD register (mm0-mm7).

        "xmm"
            A 128-bit XMM SIMD register (xmm0-xmm31).

        "xmm{k}"
            A 128-bit XMM SIMD register (xmm0-xmm31), optionally merge-masked by an AVX-512 mask register (k1-k7).

        "xmm{k}{z}"
            A 128-bit XMM SIMD register (xmm0-xmm31), optionally masked by an AVX-512 mask register (k1-k7).

        "ymm"
            A 256-bit YMM SIMD register (ymm0-ymm31).

        "ymm{k}"
            A 256-bit YMM SIMD register (ymm0-ymm31), optionally merge-masked by an AVX-512 mask register (k1-k7).

        "ymm{k}{z}"
            A 256-bit YMM SIMD register (ymm0-ymm31), optionally masked by an AVX-512 mask register (k1-k7).

        "zmm"
            A 512-bit ZMM SIMD register (zmm0-zmm31).

        "zmm{k}"
            A 512-bit ZMM SIMD register (zmm0-zmm31), optionally merge-masked by an AVX-512 mask register (k1-k7).

        "zmm{k}{z}"
            A 512-bit ZMM SIMD register (zmm0-zmm31), optionally masked by an AVX-512 mask register (k1-k7).

        "k"
            An AVX-512 mask register (k0-k7).

        "k{k}"
            An AVX-512 mask register (k0-k7), optionally merge-masked by an AVX-512 mask register (k1-k7).

        "m"
            A memory operand of any size.

        "m8"
            An 8-bit memory operand.

        "m16"
            A 16-bit memory operand.

        "m16{k}{z}"
            A 16-bit memory operand, optionally masked by an AVX-512 mask register (k1-k7).

        "m32"
            A 32-bit memory operand.

        "m32{k}"
            A 32-bit memory operand, optionally merge-masked by an AVX-512 mask register (k1-k7).

        "m32{k}{z}"
            A 32-bit memory operand, optionally masked by an AVX-512 mask register (k1-k7).

        "m64"
            A 64-bit memory operand.

        "m64{k}"
            A 64-bit memory operand, optionally merge-masked by an AVX-512 mask register (k1-k7).

        "m64{k}{z}"
            A 64-bit memory operand, optionally masked by an AVX-512 mask register (k1-k7).

        "m80"
            An 80-bit memory operand.

        "m128"
            A 128-bit memory operand.

        "m128{k}{z}"
            A 128-bit memory operand, optionally masked by an AVX-512 mask register (k1-k7).

        "m256"
            A 256-bit memory operand.

        "m256{k}{z}"
            A 256-bit memory operand, optionally masked by an AVX-512 mask register (k1-k7).

        "m512"
            A 512-bit memory operand.

        "m512{k}{z}"
            A 512-bit memory operand, optionally masked by an AVX-512 mask register (k1-k7).

        "m64/m32bcst"
            A 64-bit memory operand or a 32-bit memory operand broadcasted to 64 bits {1to2}.

        "m128/m32bcst"
            A 128-bit memory operand or a 32-bit memory operand broadcasted to 128 bits {1to4}.

        "m256/m32bcst"
            A 256-bit memory operand or a 32-bit memory operand broadcasted to 256 bits {1to8}.

        "m512/m32bcst"
            A 512-bit memory operand or a 32-bit memory operand broadcasted to 512 bits {1to16}.

        "m128/m64bcst"
            A 128-bit memory operand or a 64-bit memory operand broadcasted to 128 bits {1to2}.

        "m256/m64bcst"
            A 256-bit memory operand or a 64-bit memory operand broadcasted to 256 bits {1to4}.

        "m512/m64bcst"
            A 512-bit memory operand or a 64-bit memory operand broadcasted to 512 bits {1to8}.

        "vm32x"
            A vector of memory addresses using VSIB with 32-bit indices in XMM register.

        "vm32x{k}"
            A vector of memory addresses using VSIB with 32-bit indices in XMM register merge-masked by an AVX-512 mask
            register (k1-k7).

        "vm32y"
            A vector of memory addresses using VSIB with 32-bit indices in YMM register.

        "vm32y{k}"
            A vector of memory addresses using VSIB with 32-bit indices in YMM register merge-masked by an AVX-512 mask
            register (k1-k7).

        "vm32z"
            A vector of memory addresses using VSIB with 32-bit indices in ZMM register.

        "vm32z{k}"
            A vector of memory addresses using VSIB with 32-bit indices in ZMM register merge-masked by an AVX-512 mask
            register (k1-k7).

        "vm64x"
            A vector of memory addresses using VSIB with 64-bit indices in XMM register.

        "vm64x{k}"
            A vector of memory addresses using VSIB with 64-bit indices in XMM register merge-masked by an AVX-512 mask
            register (k1-k7).

        "vm64y"
            A vector of memory addresses using VSIB with 64-bit indices in YMM register.

        "vm64y{k}"
            A vector of memory addresses using VSIB with 64-bit indices in YMM register merge-masked by an AVX-512 mask
            register (k1-k7).

        "vm64z"
            A vector of memory addresses using VSIB with 64-bit indices in ZMM register.

        "vm64z{k}"
            A vector of memory addresses using VSIB with 64-bit indices in ZMM register merge-masked by an AVX-512 mask
            register (k1-k7).

        "{sae}"
            Suppress-all-exceptions modifier. Can be omitted.

        "{er}"
            Embedded rounding control. Can be omitted.

    :ivar is_input: indicates if the instruction reads the variable specified by this operand.
    :ivar is_output: indicates if the instruction writes the variable specified by this operand.
    :ivar extended_size: for immediate operands the size of the value in bytes after size-extension.

        The extended size affects which operand values can be encoded. E.g. a signed imm8 operand 
        would normally values in the [-128, 127] range. But if it is extended to 4 bytes, it can also
        encode values in [2**32 - 128, 2**32 - 1] range.
    """
    def __init__(self, type):
        self.type = type
        self.is_input = False
        self.is_output = False
        self.extended_size = None

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
        return self.type in ["al", "cl", "ax", "eax", "xmm0", "r8", "r16", "r32", "mm", "xmm", "ymm", "zmm", "k"]

    @property
    def is_memory(self):
        """Indicates whether this operand specifies a memory location"""
        return self.type in ["m", "m8", "m16", "m32", "m64", "m80", "m128", "m256", "m512", "vm32x", "vm32y", "vm64x", "vm64y"]

    @property
    def is_immediate(self):
        """Indicates whether this operand is an immediate constant"""
        return self.type in ["imm4", "imm8", "imm16", "imm32"]


class ISAExtension:
    _score_map = {
        "CPUID": 1,
        "RDTSC": 5,
        "RDTSCP": 6,
        "CMOV": 20,
        "POPCNT": 100,
        "LZCNT": 101,
        "TBM": 102,
        "BMI": 103,
        "BMI2": 104,
        "ADX": 105,
        "MMX": 30,
        "MMX+": 31,
        "FEMMS": 40,
        "3dnow!": 41,
        "3dnow!+": 42,
        "SSE": 50,
        "SSE2": 51,
        "SSE3": 52,
        "SSSE3": 53,
        "SSE4A": 54,
        "SSE4.1": 55,
        "SSE4.2": 56,
        "FMA3": 60,
        "FMA4": 61,
        "XOP": 62,
        "F16C": 63,
        "AVX": 70,
        "AVX2": 71,
        "AVX512F": 72,
        "AVX512BW": 73,
        "AVX512DQ": 74,
        "AVX512VL": 75,
        'AVX512PF': 76,
        'AVX512ER': 77,
        'AVX512CD': 78,
        'AVX512VBMI': 79,
        'AVX512IFMA': 80,
        "RDRAND": 85,
        "RDSEED": 86,
        "PCLMULQDQ": 90,
        "AES": 91,
        "SHA": 92,
    }

    """An extension to x86 instruction set.

    :ivar name: name of the ISA extension. Possible values are:

        - "RDTSC"     := The `RDTSC` instruction.
        - "RDTSCP"    := The `RDTSCP` instruction.
        - "CPUID"     := The `CPUID` instruction.
        - "FEMMS"     := The `FEMMS` instruction.
        - "POPCNT"    := The `POPCNT` instruction.
        - "LZCNT"     := The `LZCNT` instruction.
        - "PCLMULQDQ" := The `PCLMULQDQ` instruction.
        - "RDRAND"    := The `RDRAND` instruction.
        - "RDSEED"    := The `RDSEED` instruction.
        - "CMOV"      := Conditional MOVe instructions.
        - "MMX"       := MultiMedia eXtension.
        - "MMX+"      := AMD MMX+ extension / Integer SSE (Intel).
        - "3dnow!"    := AMD 3dnow! extension.
        - "3dnow+!"   := AMD 3dnow!+ extension.
        - "SSE"       := Streaming SIMD Extension.
        - "SSE2"      := Streaming SIMD Extension 2.
        - "SSE3"      := Streaming SIMD Extension 3.
        - "SSSE3"     := Supplemental Streaming SIMD Extension 3.
        - "SSE4.1"    := Streaming SIMD Extension 4.1.
        - "SSE4.2"    := Streaming SIMD Extension 4.2.
        - "SSE4A"     := Streaming SIMD Extension 4a.
        - "AVX"       := Advanced Vector eXtension.
        - "AVX2"      := Advanced Vector eXtension 2.
        - "AVX512F"    := AVX-512 Foundation instructions.
        - "AVX512BW"   := AVX-512 Byte and Word instructions.
        - "AVX512DQ"   := AVX-512 Doubleword and Quadword instructions.
        - "AVX512VL"   := AVX-512 Vector Length extension (EVEX-encoded XMM/YMM operations).
        - "AVX512PF"   := AVX-512 Prefetch instructions.
        - "AVX512ER"   := AVX-512 Exponential and Reciprocal instructions.
        - "AVX512CD"   := AVX-512 Conflict Detection instructions.
        - "AVX512VBMI" := AVX-512 Vector Bit Manipulation instructions.
        - "AVX512IFMA" := AVX-512 Integer 52-bit Multiply-Accumulate instructions.
        - "XOP"       := eXtended OPerations extension.
        - "F16C"      := Half-Precision (F16) Conversion instructions.
        - "FMA3"      := Fused Multiply-Add instructions (3-operand).
        - "FMA4"      := Fused Multiply-Add instructions (4-operand).
        - "BMI"       := Bit Manipulation Instructions.
        - "BMI2"      := Bit Manipulation Instructions 2.
        - "TBM"       := Trailing Bit Manipulation instructions.
        - "ADX"       := The `ADCX` and `ADOX` instructions.
        - "AES"       := `AES` instruction set.
        - "SHA"       := `SHA` instruction set.
    """

    def __init__(self, name):
        self.name = name

    def __str__(self):
        """Returns the name of the ISA extension"""
        return self.name

    def __repr__(self):
        return str(self)

    def __eq__(self, other):
        return isinstance(other, ISAExtension) and self.name == other.name

    def __ne__(self, other):
        return not isinstance(other, ISAExtension) or self.name != other.name

    def __hash__(self):
        return hash(self.name)

    @property
    def score(self):
        """A number that can be used to order a list of ISA extensions"""
        return self._score_map.get(self.name, 0)


class Encoding:
    """Instruction encoding

    :ivar components: a list of :class:`Prefix`, :class:`VEX`, :class:`Opcode`, :class:`ModRM`, \
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


class VEX:
    """VEX or XOP prefix.

    VEX and XOP prefixes use the same format and differ only by leading byte.
    The `type` property helps to differentiate between the two prefix types.

    Encoding may have only one VEX prefix and if present, it immediately precedes the opcode, and no other prefix is \
    allowed.

    :ivar type: the type of the leading byte for VEX encoding. Possible values are:

        "VEX"
            The VEX prefix (0xC4 or 0xC5) is used.

        "XOP"
            The XOP prefix (0x8F) is used.

    :ivar mmmmm: the VEX m-mmmm (implied leading opcode bytes) field. In AMD documentation this field is called map_select. Possible values are:

        0b00001
            Implies 0x0F leading opcode byte.

        0b00010
            Implies 0x0F 0x38 leading opcode bytes.

        0b00011
            Implies 0x0F 0x3A leading opcode bytes.

        0b01000
            This value does not have opcode byte interpretation. Only XOP instructions use this value.

        0b01001
            This value does not have opcode byte interpretation. Only XOP and TBM instructions use this value.

        0b01010
            This value does not have opcode byte interpretation. Only TBM instructions use this value.

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

    :ivar L: the VEX.L bit. Possible values are 0, 1, and None.

        None indicates that the bit is ignored.

    :ivar R: the VEX.R bit. Always equals 0 in 32-bit x86 architecture.

    :ivar B: the VEX.B bit. Always equals 0 in 32-bit x86 architecture.

    :ivar X: the VEX.X bit. Always equals 0 in 32-bit x86 architecture.

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

    def set_ignored(self, w=0, l=0):
        """Sets values for ignored bits

        :param int w: the value (0 or 1) to be assigned to VEX.W bit if it is ignored.
        :param int l: the value (0 or 1) to be assigned to VEX.L bit if it is ignored.
        """
        assert w in [0, 1], "VEX.W can be only 0 or 1"
        assert l in [0, 1], "VEX.L can be only 0 or 1"
        if self.W is None:
            self.W = w
        if self.L is None:
            self.L = l


class EVEX:
    """EVEX prefix.

    Encoding may have only one EVEX prefix and if present, it immediately precedes the opcode, and no other prefix is \
    allowed.

    :ivar mm: the EVEX mm (compressed legacy escape) field. Identical to two low bits of VEX.m-mmmm field. Possible
    values are:

        0b01
            Implies 0x0F leading opcode byte.

        0b10
            Implies 0x0F 0x38 leading opcode bytes.

        0b11
            Implies 0x0F 0x3A leading opcode bytes.

    :ivar pp: the EVEX pp (compressed legacy prefix) field. Possible values are:

        0b00
            No implied prefix.

        0b01
            Implied 0x66 prefix.

        0b10
            Implied 0xF3 prefix.

        0b11
            Implied 0xF2 prefix.

    :ivar W: the EVEX.W bit. Possible values are 0, 1, and None.

        None indicates that the bit is ignored.

    :ivar LL: the EVEX.L'L bits. Specify either vector length for the operation, or explicit rounding control (in which
    case operation is 512 bits wide). Possible values:

        None
            Indicates that the EVEX.L'L field is ignored.

        0b00
            128-bits wide operation.

        0b01
            256-bits wide operation.

        0b10
            512-bits wide operation.

        Reference to the last instruction operand
            EVEX.L'L are interpreted as rounding control and set to the value specified by the operand. If the rounding
            control operand is omitted, EVEX.L'L is set to 0b10 (embedded rounding control is only supported for 512-bit
            wide operations).

    :ivar RR: the EVEX.R'R bits. Always equals 0b00 in 32-bit x86 architecture.

    :ivar B: the EVEX.B bit. Always equals 0 in 32-bit x86 architecture.

    :ivar X: the EVEX.X bit. Always equals 0 in 32-bit x86 architecture.

    :ivar vvvv: the EVEX vvvv field. Possible values are 0b0000 or a reference to one of the instruction operands.

        The value 0b0000 indicates that this field is not used. \
        If vvvv is a reference to an instruction operand, the operand is of register type and EVEX.vvvv field specifies\
        its number.

    :ivar V: the EVEX V field. Always equals 0 in 32-bit x86 architecture.

    :ivar b: the EVEX b (broadcast/rounding control/suppress all exceptions context) bit. Possible values are 0 or 1.

    :ivar aaa: the EVEX aaa (embedded opmask register specifier) field. Possible values are 0 or a reference to one of
    the instruction operands.

        The value 0 indicates that this field is not used. \
        If aaa is a reference to an instruction operand, the operand supports register mask, and EVEX.aaa encodes the \
        mask register.

    :ivar z: the EVEX z bit. Possible values are None, 0 or a reference to one of the instruction operands.

        None indicates that the bit is ignored. \
        The value 0 indicates that the bit is not used. \
        If aaa is a reference to an instruction operand, the operand supports zero-masking with register mask, and
        EVEX.z indicates whether zero-masking is used.

    :ivar disp8xN: the N value used for encoding compressed 8-bit displacement. Possible values are powers of 2 in \
    [1, 64] range or None.

        None indicates that this instruction form does not use displacement (the form has no memory operands).
    """

    def __init__(self):
        self.mm = None
        self.pp = None
        self.W = None
        self.LL = None
        self.RR = None
        self.B = None
        self.X = None
        self.vvvv = None
        self.V = None
        self.b = None
        self.aaa = None
        self.z = None
        self.disp8xN = None

    def set_ignored(self, w=0, ll=0, z=0):
        """Sets values for ignored bits

        :param int w: the value (0 or 1) to be assigned to EVEX.W bit if it is ignored.
        :param int ll: the value (0b00, 0b01, 0b10, or 0b11) to be assigned to EVEX.L'L field if it is ignored.
        :param int z: the value (0 or 1) to be assigned to EVEX.z bit if it is ignored.
        """
        assert w in [0, 1], "EVEX.W can be only 0 or 1"
        assert ll in [0b00, 0b01, 0b10, 0b11], "EVEX.L'L must be a 2-bit integer value"
        assert z in [0, 1], "EVEX.z can be only 0 or 1"
        if self.W is None:
            self.W = w
        if self.LL is None:
            self.LL = ll
        if self.z is None:
            self.z = z


class Opcode:
    """Operation code

    Encoding may include more than one opcode. Opcodes do not necessarily go in sequence.

    :ivar byte: operation code as a byte integer (0 <= `byte` <= 255)
    :ivar addend: None or a reference to an instruction operand.

        If addend is a reference to an instruction operand, the operand is of register type and the three lowest bits
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

    :ivar size: size of the constant in bytes. Possible values are 1, 2, or 4.
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

    :ivar size: size of the offset in bytes. Always equals 4.
    :ivar value: value of the offset. Must be a reference to an instruction operand.

        The instruction operand has "moffs" type of the matching size.
    """
    def __init__(self):
        self.size = None
        self.value = None


def _bool(xml_boolean):
    """Converts strings "true" and "false" from XML files to Python bool"""

    assert xml_boolean in ["true", "false"], \
        "The boolean string must be \"true\" or \"false\""
    return {"true": True, "false": False}[xml_boolean]


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


def read_instruction_set(filename=os.path.join(os.path.dirname(os.path.abspath(__file__)), "x86.xml")):
    """Reads instruction set data from an XML file and returns a list of :class:`Instruction` objects

    :param filename: path to an XML file with instruction set data
    """
    xml_tree = ET.parse(filename)
    xml_instruction_set = xml_tree.getroot()
    assert xml_instruction_set.tag == "InstructionSet"
    assert xml_instruction_set.attrib["name"] == "x86"

    instruction_set = []
    for xml_instruction in xml_instruction_set:
        assert xml_instruction.tag == "Instruction"
        instruction = Instruction(xml_instruction.attrib["name"])
        instruction.summary = xml_instruction.attrib["summary"]
        for xml_instruction_form in xml_instruction:
            instruction_form = InstructionForm(instruction.name)
            instruction_form.gas_name = xml_instruction_form.attrib["gas-name"]
            instruction_form.go_name = xml_instruction_form.attrib.get("go-name")
            instruction_form.mmx_mode = xml_instruction_form.attrib.get("mmx-mode")
            instruction_form.xmm_mode = xml_instruction_form.attrib.get("xmm-mode")
            instruction_form.cancelling_inputs = _bool(xml_instruction_form.attrib.get("cancelling-inputs", "false"))
            for xml_operand in xml_instruction_form.findall("Operand"):
                operand = Operand(xml_operand.attrib["type"])
                operand.is_input = _bool(xml_operand.attrib.get("input", "false"))
                operand.is_output = _bool(xml_operand.attrib.get("output", "false"))
                if "extended-size" in xml_operand.attrib:
                    operand.extended_size = int(xml_operand.attrib.get("extended-size"))
                instruction_form.operands.append(operand)
            for xml_implicit_operand in xml_instruction_form.findall("ImplicitOperand"):
                if _bool(xml_implicit_operand.attrib["input"]):
                    instruction_form.implicit_inputs.add(xml_implicit_operand.attrib["id"])
                if _bool(xml_implicit_operand.attrib["output"]):
                    instruction_form.implicit_outputs.add(xml_implicit_operand.attrib["id"])
            for xml_isa_extension in xml_instruction_form.findall("Extension"):
                assert "id" in xml_isa_extension.attrib
                isa_extension = ISAExtension(xml_isa_extension.attrib["id"])
                instruction_form.isa_extensions.append(isa_extension)
            for xml_encoding in xml_instruction_form.findall("Encoding"):
                encoding = Encoding()
                for xml_component in xml_encoding:
                    if xml_component.tag == "Prefix":
                        prefix = Prefix()
                        prefix.byte = _parse_value(xml_component.attrib.get("byte"), [], 16)
                        assert prefix.byte in [0x66, 0xF2, 0xF3]
                        prefix.is_mandatory = _parse_boolean(xml_component.attrib.get("mandatory"))
                        encoding.components.append(prefix)
                    elif xml_component.tag == "VEX":
                        vex = VEX()

                        vex.type = xml_component.attrib["type"]
                        vex.pp = int(xml_component.attrib["pp"], 2)
                        vex.mmmmm = int(xml_component.attrib["m-mmmm"], 2)
                        vex.W = _parse_value(xml_component.attrib.get("W"), [], 2)
                        vex.L = _parse_value(xml_component.attrib.get("L"), [], 2)
                        vex.R = _parse_value(xml_component.attrib.get("R"), instruction_form.operands, 2)
                        vex.B = _parse_value(xml_component.attrib.get("B"), instruction_form.operands, 2)
                        vex.X = _parse_value(xml_component.attrib.get("X"), instruction_form.operands, 2)
                        vex.vvvv = _parse_value(xml_component.attrib.get("vvvv"), instruction_form.operands, 2)

                        encoding.components.append(vex)
                    elif xml_component.tag == "EVEX":
                        evex = EVEX()
                        evex.pp = int(xml_component.attrib["pp"], 2)
                        evex.mm = int(xml_component.attrib["mm"], 2)
                        evex.W = _parse_value(xml_component.attrib.get("W"), [], 2)
                        evex.LL = _parse_value(xml_component.attrib.get("LL"), instruction_form.operands, 2)
                        evex.vvvv = _parse_value(xml_component.attrib["vvvv"], instruction_form.operands, 2)
                        evex.V = _parse_value(xml_component.attrib["V"], instruction_form.operands, 2)
                        evex.aaa = _parse_value(xml_component.attrib.get("aaa", "000"), instruction_form.operands, 2)
                        evex.z = _parse_value(xml_component.attrib.get("z", "0"), instruction_form.operands, 2)
                        evex.X = _parse_value(xml_component.attrib.get("X"), instruction_form.operands, 2)
                        evex.B = _parse_value(xml_component.attrib["B"], instruction_form.operands, 2)
                        evex.RR = _parse_value(xml_component.attrib.get("RR"), instruction_form.operands, 2)
                        evex.b = _parse_value(xml_component.attrib["b"], instruction_form.operands, 2)
                        evex.disp8xN = _parse_value(xml_component.attrib.get("disp8xN"), [], 10)
                        encoding.components.append(evex)
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
                    elif xml_component.tag == "RegisterByte":
                        register_byte = RegisterByte()
                        register_byte.register = _parse_value(xml_component.attrib.get("register"), instruction_form.operands)
                        register_byte.payload = _parse_value(xml_component.attrib.get("payload"), instruction_form.operands)
                        encoding.components.append(register_byte)
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
