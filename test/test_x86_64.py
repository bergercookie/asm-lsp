import unittest


class ReadXML(unittest.TestCase):
    def runTest(self):
        from opcodes.x86_64 import read_instruction_set
        instruction_set = read_instruction_set()

