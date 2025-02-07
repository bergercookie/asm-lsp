import xml.etree.ElementTree as ET
import xml.dom.minidom as minidom

from pdfplumber import table
from core.data_processing import InstructionForm


class InstructionXMLBuilder:
    status_registers = ['I', 'T', 'H', 'S', 'V', 'N', 'Z', 'C']

    @classmethod
    def create_instruction_xml(cls, instruction_name: str, instruction_forms: list[InstructionForm]) -> ET.Element:
        root = ET.Element('Instruction')
        root.set('name', instruction_name)

        for form in instruction_forms:
            instruction_form = cls.create_instruction_form_xml(form)
            instruction_form_version = instruction_form.get('value')
            if instruction_form_version is None:
                raise RuntimeError(f'form aspect: {form} instruction form version is None')
            instruction_form_content = instruction_form.find('InstructionForm')
            if instruction_form_content is None:
                raise RuntimeError(f'form aspect: {form} instruction form content is None')

            instruction_version = next((version for version in root.findall('Version') if version.get('value') == instruction_form_version), None)
            if instruction_version is None:
                instruction_version = ET.SubElement(root, 'Version')
                instruction_version.set('value', instruction_form_version)

            instruction_version.append(instruction_form_content)

        # sorting mnemonics
        for version in root:
            sorted_instruction_forms = sorted(version, key=lambda instruction_form: str(instruction_form.get('mnemonic')))
            for elem in version.findall('*'):
                version.remove(elem)
            version.extend(sorted_instruction_forms)

        # sorting versions
        sorted_versions = sorted(root, key=lambda version: str(version.get('value')))
        for version in root.findall('*'):
            root.remove(version)
        root.extend(sorted_versions)

        return root


    @classmethod
    def create_instruction_form_xml(cls, form: InstructionForm) -> ET.Element:
        if not cls.are_instruction_form_aspects_valid(form):
            raise RuntimeError(f'form aspects not valid: {form}')

        root = ET.Element('Version')
        version = form['version']
        if version:
            root.set('value', version)

        form_et = ET.SubElement(root, 'InstructionForm')
        mnemonic = form['mnemonic']
        if mnemonic:
            form_et.set('mnemonic', mnemonic)

        table_sections = form['table_data']
        if table_sections:
            if table_sections['Description']:
                form_et.set('summary', str(table_sections['Description']))

            if table_sections['Operands']:
                for operand in table_sections['Operands']:
                    operand_et = ET.SubElement(form_et, 'Operand')
                    operand_et.set('type', operand)

            if table_sections['Clocks']:
                clocks_et = ET.SubElement(form_et, 'Clocks')
                for key, value in table_sections['Clocks'].items():
                    clock_et = ET.SubElement(clocks_et, str(key))
                    clock_et.set('value', value)

        chapter_sections = form['chapter_data']
        if chapter_sections:
            if chapter_sections['SREG']:
                sreg = ET.SubElement(form_et, 'SREG')
                for i in range(0, len(cls.status_registers)):
                    flag_et = ET.SubElement(sreg, cls.status_registers[i])
                    flag_et.set('value', chapter_sections['SREG'][0][i])

            if chapter_sections['Opcode']:
                encoding_et = ET.SubElement(form_et, 'Encoding')
                for bits in chapter_sections['Opcode']:
                    opcode_niddle_et = ET.SubElement(encoding_et, 'Opcode')
                    opcode_niddle_et.set('nibble', bits)

        return root


    @classmethod
    def are_instruction_form_aspects_valid(cls, form_aspects: dict) -> bool:
        for aspect_name in InstructionForm.valid_keys:
            if (aspect_name not in form_aspects.keys()):
                return False

        return True


    @classmethod
    def are_instruction_form_aspects_filled(cls, form_aspects: dict) -> bool:
        for aspect_name in InstructionForm.valid_keys:
            aspect = form_aspects[aspect_name]

            match aspect_name:
                case 'mnemonic'|'version':
                    if not isinstance(aspect, str):
                        return False

                case 'table_data':
                    aspect_sections = ['Description', 'Operands', 'Clocks']
                    if (not isinstance(aspect, dict) or
                        any(section not in aspect.keys() for section in aspect_sections)):

                        return False

                case 'chapter_data':
                    aspect_sections = ['Opcode', 'SREG']
                    if (not isinstance(aspect, dict) or
                        any(section not in aspect.keys() for section in aspect_sections)):

                        return False
        return True


    @classmethod
    def format_et_to_str(cls, tree: ET.ElementTree) -> str:
        xml_str = ET.tostring(tree.getroot(), encoding='utf-8', xml_declaration=True).decode('utf-8')

        formatted_xml_str = minidom.parseString(xml_str).toprettyxml(indent="  ")
        return formatted_xml_str


    @classmethod
    def create_instruction_form_str(cls, form: InstructionForm) -> str:
        form_el = cls.create_instruction_form_xml(form)
        form_tree = ET.ElementTree(form_el)
        form_str = cls.format_et_to_str(form_tree)
        return form_str


    @classmethod
    def create_instruction_str(cls, name: str, instruction_forms: list[InstructionForm]) -> str:
        instruction = cls.create_instruction_xml(name, instruction_forms)
        instruction_tree = ET.ElementTree(instruction)
        instruction_str = cls.format_et_to_str(instruction_tree)
        return instruction_str
