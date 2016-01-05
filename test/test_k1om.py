import unittest


class ReadXML(unittest.TestCase):
    def runTest(self):
        from opcodes.k1om import read_instruction_set
        instruction_set = read_instruction_set()

