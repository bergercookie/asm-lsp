from __future__ import print_function
import xml.etree.ElementTree as ET
import os


class Instruction:
    def __init__(self, name):
        self.name = name
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
        None if instruction is not supported in Go/Plan 9 assembler.
    :ivar operands: a list of :class:`Operand` objects representing the instruction operands.
    :ivar isa_extensions: a list of :class:`ISAExtension` objects that represent the ISA extensions required to execute
        the instruction.
    :ivar encodings: a list of :class:`Encoding` objects representing the possible encodings for this instruction.
    """

    def __init__(self, name):
        self.name = name
        self.gas_name = None
        self.go_name = None
        self.operands = []
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
    def __init__(self, type):
        self.type = type
        self.is_input = False
        self.is_output = False

    def __str__(self):
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
        return self.is_input or self.is_output

    @property
    def is_register(self):
        return self.type in {"al", "cl", "ax", "eax", "rax", "xmm0", "r8", "r16", "r32", "r64", "r8l", "r16l", "r32l", "mm", "xmm", "ymm"}

    @property
    def is_memory(self):
        return self.type in {"m", "m8", "m16", "m32", "m64", "m80", "m128", "m256", "m512"}


class ISAExtension:
    def __init__(self, name):
        self.name = name

    def __str__(self):
        return self.name


class Encoding:
    """Instruction encoding

    :ivar components: a list of :class:`Prefix`, :class:`REX`, :class:`VEX`, :class:`Opcode`, :class:`ModRM`, \
        :class:`ImmediateByte`, :class:`ImmediateWord`, :class:`ImmediateDWord`, :class:`ImmediateQWord`, \
        :class:`DataOffset32`, :class:`DataOffset64`, :class:`CodeOffset8`, or :class:`CodeOffset32` objects that \
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

    def __init__(self, byte, is_mandatory):
        self.byte = byte
        self.is_mandatory = is_mandatory


class REX:
    """REX prefix.

    Encoding may have only one REX prefix and if present, it immediately precedes the opcode.

    :ivar is_mandatory: indicates whether the REX prefix must be encoded even if no extended registers are used. \
        REX is mandatory for most 64-bit instructions (encoded with REX.W = 1) and instructions that operate on the \
        extended set of 8-bit registers (to indicate access to dil/sil/bpl/spl as opposed to ah/bh/ch/dh which use the \
        same ModR/M).
    :ivar W: the REX.W bit. Possible values are 0, 1, and None. None indicates that the bit is ignored.
    :ivar R: the REX.R bit. Possible values are 0, 1, None, or a reference to one of the instruction operands. \
        The value None indicates that this bit is ignored. \
        If R is a reference to an instruction operand, the operand is of register type and REX.R bit specifies the \
        high bit (bit 3) of the register number.
    :ivar B: the REX.B bit. Possible values are 0, 1, None, or a reference to one of the instruction operands. \
        The value None indicates that this bit is ignored. \
        If R is a reference to an instruction operand, the operand can be of register or memory type. If the operand \
        is of register type, the REX.R bit specifies the high bit (bit 3) of the register number, and the REX.X bit is \
        ignored. If the operand is of memory type, the REX.R bit specifies the high bit (bit 3) of the base register \
        number, and the X instance variable refers to the same operand.
    :ivar X: the REX.X bit. Possible values are 0, 1, None, or a reference to one of the instruction operands. \
        The value None indicates that this bit is ignored. \
        If X is a reference to an instruction operand, the operand is of memory type and the REX.X bit specifies the \
        high bit (bit 3) of the index register number, and the B instance variable refers to the same operand.
    """

    def __init__(self, is_mandatory):
        self.is_mandatory = is_mandatory
        self.W = None
        self.R = None
        self.X = None
        self.B = None

    def set_ignored(self, w=0, r=0, x=0, b=0):
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

    :ivar mmmmm: the VEX m-mmmm (implied leading opcode bytes) field. Possible values are 0b00001 (implies 0x0F \
        leading opcode byte), 0b00010 (implies 0x0F 0x38 leading opcode bytes), and 0b00011 (implies 0x0F 0x3A leading \
        opcode bytes). Only VEX prefix with m-mmmm equal to 0b00001 could be encoded in two bytes.
    :ivar pp: the VEX pp (implied legacy prefix) field. Possible values are 0b00 (no implied prefix), 0b01 (implied \
        0x66 prefix), 0b10 (implied 0xF3 prefix), and 0b11 (implied 0xF2 prefix).
    :ivar W: the VEX.W bit. Possible values are 0, 1, and None. None indicates that the bit is ignored.
    :ivar L: the VEX.L bit. Possible values are 0, 1, and None. None indicates that the bit is ignored.
    :ivar R: the VEX.R bit. Possible values are 0, 1, None, or a reference to one of the instruction operands. \
        The value None indicates that this bit is ignored. \
        If R is a reference to an instruction operand, the operand is of register type and VEX.R bit specifies the \
        high bit (bit 3) of the register number.
    :ivar B: the VEX.B bit. Possible values are 0, 1, None, or a reference to one of the instruction operands. \
        The value None indicates that this bit is ignored. \
        If R is a reference to an instruction operand, the operand can be of register or memory type. If the operand is \
        of register type, the VEX.R bit specifies the high bit (bit 3) of the register number, and the VEX.X bit is \
        ignored. If the operand is of memory type, the VEX.R bit specifies the high bit (bit 3) of the base register \
        number, and the X instance variable refers to the same operand.
    :ivar X: the VEX.X bit. Possible values are 0, 1, None, or a reference to one of the instruction operands. \
        The value None indicates that this bit is ignored. \
        If X is a reference to an instruction operand, the operand is of memory type and the VEX.X bit specifies the \
        high bit (bit 3) of the index register number, and the B instance variable refers to the same operand.
    :ivar vvvv: the VEX vvvv field. Possible values are 0b0000 to 0b1111, None, or a reference to one of the \
        instruction operands. \
        The value None indicates that this bit is not used. Then it must be encoded as 0b1111. \
        If vvvv is a reference to an instruction operand, the operand is of register type and VEX.vvvv field specifies \
        its number.
    """

    def __init__(self):
        self.mmmmm = None
        self.pp = None
        self.W = None
        self.L = None
        self.R = None
        self.X = None
        self.B = None
        self.vvvv = None

    def set_ignored(self, w=0, l=0, r=0, x=0, b=0, vvvv=0b1111):
        if self.W is None:
            self.W = w
        if self.L is None:
            self.L = l
        if self.R is None:
            self.R = r
        if self.X is None:
            self.X = x
        if self.B is None:
            self.B = b
        if self.vvvv is None:
            self.vvvv = vvvv


class Opcode:
    def __init__(self, byte):
        self.byte = byte
        self.addend = None

    def __str__(self):
        return "0x%02X".format(self.byte)

    def __repr__(self):
        return str(self)


class ModRM:
    def __init__(self):
        self.mode = None
        self.rm = None
        self.reg = None


class ImmediateByte:
    def __init__(self):
        self.byte = None


class ImmediateWord:
    def __init__(self):
        self.word = None


class ImmediateDWord:
    def __init__(self):
        self.dword = None


class ImmediateQWord:
    def __init__(self):
        self.qword = None


class CodeOffset32:
    def __init__(self):
        self.offset = None


class CodeOffset8:
    def __init__(self):
        self.offset = None


class DataOffset32:
    def __init__(self):
        self.offset = None


class DataOffset64:
    def __init__(self):
        self.offset = None


def read_instruction_set(filename=os.path.join(os.path.dirname(os.path.abspath(__file__)), "x86_64.xml")):
    xml_tree = ET.parse(filename)
    xml_instruction_set = xml_tree.getroot()
    assert xml_instruction_set.tag == "InstructionSet"
    assert xml_instruction_set.attrib["name"] == "x86-64"

    instruction_set = []
    for xml_instruction in xml_instruction_set:
        assert xml_instruction.tag == "Instruction"
        instruction = Instruction(xml_instruction.attrib["name"])
        for xml_instruction_form in xml_instruction:
            instruction_form = InstructionForm(instruction.name)
            instruction_form.gas_name = xml_instruction_form.attrib["gas-name"]
            if "go-name" in xml_instruction_form.attrib:
                instruction_form.go_name = xml_instruction_form.attrib["go-name"]
            for xml_operand in xml_instruction_form.findall("Operands/Operand"):
                assert "type" in xml_operand.attrib
                operand = Operand(xml_operand.attrib["type"])
                if "input" in xml_operand.attrib:
                    operand.is_input = {"true": True, "false": False}[xml_operand.attrib["input"]]
                if "output" in xml_operand.attrib:
                    operand.is_output = {"true": True, "false": False}[xml_operand.attrib["output"]]
                if "fixed" in xml_operand.attrib:
                    operand.fixed = xml_operand.attrib["fixed"]
                instruction_form.operands.append(operand)
            for xml_isa_extension in xml_instruction_form.findall("ISA/Extension"):
                assert "id" in xml_isa_extension.attrib
                isa_extension = ISAExtension(xml_isa_extension.attrib["id"])
                instruction_form.isa_extensions.append(isa_extension)
            for xml_encoding in xml_instruction_form.findall("Encodings/Encoding"):
                encoding = Encoding()
                for xml_component in xml_encoding:
                    if xml_component.tag == "Prefix":
                        is_mandatory = {"true": True, "false": False}[xml_component.attrib["mandatory"]]
                        byte = int(xml_component.attrib["byte"], 16)
                        assert byte in {0x66, 0xF2, 0xF3}
                        prefix = Prefix(byte, is_mandatory)
                        encoding.components.append(prefix)
                    elif xml_component.tag == "REX":
                        assert "mandatory" in xml_component.attrib
                        is_mandatory = {"true": True, "false": False}[xml_component.attrib["mandatory"]]

                        rex = REX(is_mandatory)

                        assert "W" in xml_component.attrib
                        rex.W = xml_component.attrib["W"]
                        if rex.W == "ignored":
                            rex.W = None
                        else:
                            rex.W = int(rex.W)

                        if "R" in xml_component.attrib:
                            assert "R-operand-number" not in xml_component.attrib
                            rex.R = xml_component.attrib["R"]
                            if rex.R == "ignored":
                                rex.R = None
                            else:
                                rex.R = int(rex.R)
                        else:
                            assert "R-operand-number" in xml_component.attrib
                            rex.R = instruction_form.operands[int(xml_component.attrib["R-operand-number"])]

                        if "B" in xml_component.attrib:
                            assert "B-operand-number" not in xml_component.attrib
                            assert "BX-operand-number" not in xml_component.attrib
                            rex.B = xml_component.attrib["B"]
                            if rex.B == "ignored":
                                rex.B = None
                            else:
                                rex.B = int(rex.B)
                        elif "B-operand-number" in xml_component.attrib:
                            assert "B" not in xml_component.attrib
                            assert "BX-operand-number" not in xml_component.attrib
                            rex.B = instruction_form.operands[int(xml_component.attrib["B-operand-number"])]
                        else:
                            assert "BX-operand-number" in xml_component.attrib
                            assert "B-operand-number" not in xml_component.attrib
                            assert "B" not in xml_component.attrib
                            assert "X" not in xml_component.attrib
                            rex.B = instruction_form.operands[int(xml_component.attrib["BX-operand-number"])]

                        if "X" in xml_component.attrib:
                            assert "BX-operand-number" not in xml_component.attrib
                            rex.X = xml_component.attrib["X"]
                            if rex.X == "ignored":
                                rex.X = None
                            else:
                                rex.X = int(rex.X)
                        else:
                            assert "BX-operand-number" in xml_component.attrib
                            rex.X = instruction_form.operands[int(xml_component.attrib["BX-operand-number"])]
                        encoding.components.append(rex)
                    elif xml_component.tag == "VEX":
                        vex = VEX()

                        assert "W" in xml_component.attrib
                        vex.W = xml_component.attrib["W"]
                        if vex.W == "ignored":
                            vex.W = None
                        else:
                            vex.W = int(vex.W)

                        assert "L" in xml_component.attrib
                        vex.L = xml_component.attrib["L"]
                        if vex.L == "ignored":
                            vex.L = None
                        else:
                            vex.L = int(vex.L)

                        vex.mmmmm = int(xml_component.attrib["m-mmmm"], 2)
                        vex.pp = int(xml_component.attrib["pp"], 2)

                        if "R" in xml_component.attrib:
                            assert "R-operand-number" not in xml_component.attrib
                            vex.R = xml_component.attrib["R"]
                            if vex.R == "ignored":
                                vex.R = None
                            else:
                                vex.R = int(rex.R)
                        else:
                            assert "R-operand-number" in xml_component.attrib
                            vex.R = instruction_form.operands[int(xml_component.attrib["R-operand-number"])]

                        if "B" in xml_component.attrib:
                            assert "B-operand-number" not in xml_component.attrib
                            assert "BX-operand-number" not in xml_component.attrib
                            vex.B = xml_component.attrib["B"]
                            if vex.B == "ignored":
                                vex.B = None
                            else:
                                vex.B = int(rex.B)
                        elif "B-operand-number" in xml_component.attrib:
                            assert "B" not in xml_component.attrib
                            assert "BX-operand-number" not in xml_component.attrib
                            vex.B = instruction_form.operands[int(xml_component.attrib["B-operand-number"])]
                        else:
                            assert "BX-operand-number" in xml_component.attrib
                            assert "B-operand-number" not in xml_component.attrib
                            assert "B" not in xml_component.attrib
                            assert "X" not in xml_component.attrib
                            vex.B = instruction_form.operands[int(xml_component.attrib["BX-operand-number"])]

                        if "X" in xml_component.attrib:
                            assert "BX-operand-number" not in xml_component.attrib
                            vex.X = xml_component.attrib["X"]
                            if vex.X == "ignored":
                                vex.X = None
                            else:
                                vex.X = int(vex.X)
                        else:
                            assert "BX-operand-number" in xml_component.attrib
                            vex.X = instruction_form.operands[int(xml_component.attrib["BX-operand-number"])]

                        vex.vvvv = None
                        if "vvvv" in xml_component.attrib:
                            assert "vvvv-operand-number" not in xml_component.attrib
                            assert xml_component.attrib["vvvv"] == "1111"
                            vex.vvvv = None
                        else:
                            assert "vvvv-operand-number" in xml_component.attrib
                            vex.vvvv = instruction_form.operands[int(xml_component.attrib["vvvv-operand-number"])]

                        encoding.components.append(vex)
                    elif xml_component.tag == "Opcode":
                        opcode = Opcode(int(xml_component.attrib["byte"], 16))
                        if "addend-operand-number" in xml_component.attrib:
                            opcode.addend = instruction_form.operands[int(xml_component.attrib["addend-operand-number"])]
                        encoding.components.append(opcode)
                    elif xml_component.tag == "ModRM":
                        modrm = ModRM()
                        if "mode" in xml_component.attrib:
                            modrm.mode = int(xml_component.attrib["mode"], 2)
                            assert modrm.mode == 0b11
                        else:
                            assert "mode-operand-number" in xml_component.attrib
                            assert xml_component.attrib["mode-operand-number"] == xml_component.attrib["rm-operand-number"]
                            modrm.mode = instruction_form.operands[int(xml_component.attrib["mode-operand-number"])]
                        if "reg" in xml_component.attrib:
                            modrm.reg = int(xml_component.attrib["reg"])
                            assert 0 <= modrm.reg <= 7
                        else:
                            assert "reg-operand-number" in xml_component.attrib
                            modrm.reg = instruction_form.operands[int(xml_component.attrib["reg-operand-number"])]
                        assert "rm-operand-number" in xml_component.attrib
                        modrm.rm = instruction_form.operands[int(xml_component.attrib["rm-operand-number"])]
                        encoding.components.append(modrm)
                    elif xml_component.tag == "Immediate":
                        assert "size" in xml_component.attrib
                        immediate_size = int(xml_component.attrib["size"])
                        assert immediate_size in {1, 2, 4, 8}
                        if immediate_size == 1:
                            immediate_byte = ImmediateByte()
                            if "operand-number" in xml_component.attrib:
                                immediate_byte.byte = instruction_form.operands[int(xml_component.attrib["operand-number"])]
                            else:
                                assert False
                            encoding.components.append(immediate_byte)
                        elif immediate_size == 2:
                            immediate_word = ImmediateWord()
                            assert "operand-number" in xml_component.attrib
                            immediate_word.word = instruction_form.operands[int(xml_component.attrib["operand-number"])]
                            encoding.components.append(immediate_word)
                        elif immediate_size == 4:
                            immediate_dword = ImmediateDWord()
                            assert "operand-number" in xml_component.attrib
                            immediate_dword.dword = instruction_form.operands[int(xml_component.attrib["operand-number"])]
                            encoding.components.append(immediate_dword)
                        elif immediate_size == 8:
                            immediate_qword = ImmediateQWord()
                            assert "operand-number" in xml_component.attrib
                            immediate_qword.qword = instruction_form.operands[int(xml_component.attrib["operand-number"])]
                            encoding.components.append(immediate_qword)
                        else:
                            assert False
                    elif xml_component.tag == "CodeOffset":
                        offset_size = int(xml_component.attrib["size"])
                        assert offset_size in {1, 4}
                        if offset_size == 1:
                            code_offset = CodeOffset8()
                            code_offset.offset = instruction_form.operands[int(xml_component.attrib["operand-number"])]
                            encoding.components.append(code_offset)
                        elif offset_size == 4:
                            code_offset = CodeOffset32()
                            code_offset.offset = instruction_form.operands[int(xml_component.attrib["operand-number"])]
                            encoding.components.append(code_offset)
                        else:
                            assert False
                    elif xml_component.tag == "DataOffset":
                        offset_size = int(xml_component.attrib["size"])
                        assert offset_size in {4, 8}
                        if offset_size == 4:
                            data_offset = DataOffset32()
                            data_offset.offset = instruction_form.operands[int(xml_component.attrib["operand-number"])]
                            encoding.components.append(data_offset)
                        elif offset_size == 8:
                            data_offset = DataOffset64()
                            data_offset.offset = instruction_form.operands[int(xml_component.attrib["operand-number"])]
                            encoding.components.append(data_offset)
                        else:
                            assert False
                    else:
                        print("Unknown encoding tag: " + xml_component.tag)

                instruction_form.encodings.append(encoding)
            instruction.forms.append(instruction_form)
        instruction_set.append(instruction)
    return instruction_set
