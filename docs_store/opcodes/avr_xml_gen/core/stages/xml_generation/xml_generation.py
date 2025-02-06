#internal
from .instruction_xml_builder import InstructionXMLBuilder

#external
from core.stages import Stage 
from core.data_processing.data_management import Context, ProcessedInstructionsData
import xml.etree.ElementTree as ET
from bs4 import BeautifulSoup


class XMLGeneration(Stage):
    def execute(self) -> None:
        for name, data in self._context[ProcessedInstructionsData].items():
            print(f'{name}: {data}')
        self._build_xml()
        self.permanently_completed = True
        self._complete()


    def _build_xml(self):
        instructions_data = self._context[ProcessedInstructionsData]
        root = ET.Element('InstructionSet')
        root.set('name', 'AVR')
        for instruction_name, instruction_forms in instructions_data.items():
            instruction_xml = InstructionXMLBuilder.create_instruction_xml(instruction_name, instruction_forms)
            root.append(instruction_xml)
        tree = ET.ElementTree(root)
        tree.write('avr_instructions.xml', encoding='utf-8', xml_declaration=True)
        bs = BeautifulSoup(open('avr_instructions.xml'), 'xml')
        instructions_xml_str = bs.prettify()
        with open('avr_instructions.xml', 'w') as file:
            file.write(instructions_xml_str)
