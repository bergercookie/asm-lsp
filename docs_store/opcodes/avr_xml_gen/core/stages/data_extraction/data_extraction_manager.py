#this code is a complete MESS
#it needs to be redone


from core.data_processing import AmbiguousList, AmbiguousDict, AmbiguousFrozen, ExtractedInstructionsData, Context, SourceInfo

import pdfplumber
from pdfplumber.page import Page
from colorama import Fore, init
from collections import OrderedDict
from typing import TypeAlias, Collection, Any
import os
import re
import pickle

from core.data_processing.data_management import DataManager


class DataExtractionManager(DataManager):
    valid_requests = {'type': ('start')}
    cpu_versions = set(('AVR', 'AVRe', 'AVRe+', 'AVRxm', 'AVRxt', 'AVRrc'))

    def __init__(self, context: Context) -> None:
        self._context = context

    #Plug for interface implementation
    #Not all managers needs data for request, so ambiguity of the class should be eliminated
    def is_request_valid(self, request: dict) -> bool:
        return True

    def request(self, request) -> Any:
        if not self.is_request_valid(request):
            return 
        match request['type']:
            case 'start':
                self.__start()

    def __start(self) -> None:
        pdf_path = self._context[SourceInfo]['pdf_path']
        dump_dir = 'intermediate_data'
        os.makedirs(dump_dir, exist_ok=True)
        tables_data = process_tables_extraction(pdf_path, dump_dir, False)
        chapter_data = process_chapter_extraction(pdf_path, dump_dir, False)
        unprocessed_data = merge_instructions_data(tables_data, chapter_data)
        final_data = process_instructions_data(unprocessed_data)

        extracted_instructions_data = ExtractedInstructionsData()
        extracted_instructions_data.update(final_data)
        self._context.record(extracted_instructions_data)

        with open('intermediate_data/avr_instructions_data.txt', 'w') as data_file:
            for instruction, data in final_data.items():
                data_file.write(f'{Fore.BLUE}{instruction}{Fore.RESET}: {data}\n')
        
        with open('intermediate_data/avr_instructions_data.pkl', 'wb') as data_file:
            pickle.dump(final_data, data_file)


#MESS
#Unsafe to read

init(autoreset=True)

PartialInstructionsData: TypeAlias = AmbiguousDict[AmbiguousFrozen[str], AmbiguousList]
RawInstructionsData: TypeAlias = dict[AmbiguousFrozen[str], AmbiguousDict] #naming dead
ProcessedInstructionsData: TypeAlias = OrderedDict[str, AmbiguousDict]
TextTable: TypeAlias = list[list[str]]

cpu_versions = DataExtractionManager.cpu_versions

def extract_needed_tables(pdf_path: str, first_page: int, last_page: int) -> list[TextTable]:
    print(Fore.BLUE + f'[PROCESS]: Starting needed tables extracting from pdf')
    needed_headers = ['Mnemonic',
                      'Operands',
                      'Description',
                      '#Clocks'] #there are several values
    tables = []
    with pdfplumber.open(pdf_path) as pdf:
        for page in pdf.pages[first_page - 1:last_page]:

            for table in page.extract_tables():
                headers = table[0]

                 #specific of this datasheet
                if headers[0] is not None and re.search('continued', headers[0].lower()):
                    del table[0]
                    headers = table[0]

                fits = all(
                    #we use re.search because of #Clocks have several values
                    any(re.search(needed_header, header) for header in headers if header is not None) 
                    for needed_header in needed_headers
                )
                if (fits):
                    tables.append(table)

    print(Fore.BLUE + f'[PROCESS]: Completes needed tables extracting from pdf')
    return tables


def extract_table_data(table: TextTable) -> PartialInstructionsData:
    table_data: PartialInstructionsData = AmbiguousDict()

    headers = table[0]
    try:
        mnemonic_ind = headers.index('Mnemonic')
        operands_ind = headers.index('Operands')
        description_ind = headers.index('Description')
        clocks_inds = [i for i in range(len(headers)) if "#Clocks" in headers[i]]

    except ValueError as e:
        raise ValueError(e)

    for row in table[1:]:
        mnemonic_data = {}

        mnemonic_data['Description'] = row[description_ind]
           
        mnemonic_data['Operands'] = []
        for operand in row[operands_ind].split(', '):
            mnemonic_data['Operands'].append(operand)

        mnemonic_data['Clocks'] = {}
        for version_ind in clocks_inds:
            version = headers[version_ind].split('\n')[1]
            value_search = re.search(r'[^()]+', row[version_ind])

            version_value = None
            if value_search:
                version_value = value_search.group(0)

            mnemonic_data['Clocks'][version] = version_value

        mnemonic = AmbiguousFrozen([row[mnemonic_ind]])

        #checking for errors
        mnemonic_data_error = check_table_row_data(mnemonic_data)
        if mnemonic_data_error:
            print(Fore.RED + f'{'-'*8}{mnemonic}{'-'*8}')
            print(mnemonic_data)
            for line in mnemonic_data_error:
                print(line)

        if mnemonic not in table_data:
            table_data[mnemonic] = AmbiguousList(mnemonic_data)
        else:
            table_data[mnemonic].append(mnemonic_data)
    
    return table_data


def check_table_row_data(table_row_data: dict) -> list[str]:
    error_message: list[str] = []
    if not table_row_data:
        error_message.append(Fore.RED + f'Table row data is empty')
    else:
        needed_info = ['Operands',
                       'Description',
                       'Clocks',
                       ]
        for e in needed_info:
            if not table_row_data[e]:
                error_message.append(Fore.RED + f'Table row data has no {e} section')

    return error_message


def extract_tables_data(tables: list[TextTable]) -> PartialInstructionsData :
    data_map: PartialInstructionsData = AmbiguousDict()

    print(Fore.BLUE + f'[PROCESS]: Starting tables data extracting')
    for table in tables:
        table_data = extract_table_data(table)

        for key, ambiguous_data in table_data.items():
            if key not in data_map:
                data_map[key] = AmbiguousList()
            data_map[key].extend(ambiguous_data)
            
    print(Fore.BLUE + f'[PROCESS]: Completes tables data extracting')
    return data_map


def get_subchapters_pages(pdf_path: str, chapter_number: int, first_page: int, last_page: int, first_subchapter: int = 1) -> list[list[Page]]: #list of subchapters pages
    subchapters_pages: list[list[Page]] = []

    print(Fore.BLUE + f'[PROCESS]: Starting subchapters pages extracting from pdf')

    with pdfplumber.open(pdf_path) as pdf:
        page_ind = first_page - 1
        page = pdf.pages[page_ind]
        subchapter_number = first_subchapter

        subchapter_regex = fr'{chapter_number}\.{subchapter_number}'
        next_subchapter_beginning = None

        #find beginning of first subchapter
        while not next_subchapter_beginning and page_ind < last_page:

            for word in page.extract_words():

                match = re.match(subchapter_regex, word['text'])
                if match:
                    next_subchapter_beginning = word['top']
                    break

            if not next_subchapter_beginning:
                page_ind += 1
                page = pdf.pages[page_ind]

        #creating subchapterS_list
        while page_ind < last_page:

            if not next_subchapter_beginning:
                raise ValueError('subchapter beginning can\'t be null')

            page = pdf.pages[page_ind]
            page = page.crop((0, next_subchapter_beginning, page.width, page.height))
            subchapter_number += 1 
            subchapter_regex = fr'{chapter_number}\.{subchapter_number}' 
            subchapter_pages = []
            next_subchapter_beginning = None

            #creating subchapter_pages
            while not next_subchapter_beginning and page_ind < last_page:
                for word in page.extract_words():

                    match = re.match(subchapter_regex, word['text'])

                    if match:
                        next_subchapter_beginning = word['top']
                        crop_bbox = (0, page.bbox[1], page.width, next_subchapter_beginning)
                        page = page.crop(crop_bbox)
                        break

                subchapter_pages.append(page)

                if not next_subchapter_beginning:
                    page_ind += 1
                    page = pdf.pages[page_ind]


            subchapters_pages.append(subchapter_pages)


    print(Fore.BLUE + f'[PROCESS]: Completes subchapters pages extracting from pdf')
    return subchapters_pages


def extract_chapter_data(pdf_path: str, chapter_number: int, first_page: int, last_page: int, first_subchapter: int = 1) -> PartialInstructionsData:
    chapter_data: PartialInstructionsData = AmbiguousDict()
    subchapters_pages: list[list[Page]] = get_subchapters_pages(pdf_path, chapter_number, first_page, last_page, first_subchapter)
    roman_numbers_regex = r'\(([iv]+)\)(-\(([iv]+)\))?'
    status_registers = ['I', 'T', 'H', 'S', 'V', 'N', 'Z', 'C'] 

    print(Fore.BLUE + f'[PROCESS]: Starting chapter data extracting')

    for subchapter_pages in subchapters_pages:
        subchapter_data: AmbiguousList[Collection] = AmbiguousList()
        raw_data: dict[str, AmbiguousList[list]] = {
            'Opcode': AmbiguousList(),
            'SREG': AmbiguousList(),
        }
        interrupted_flag = False #special for 67 page. Why didn't you make ....continued???

        for page in subchapter_pages:
            for table in page.extract_tables():

                if any(section is None for row in table for section in row):
                    continue

                headers = table[0]
                if headers[0] is not None and re.search('continued', headers[0].lower()): #finding continuation of table
                    del table[0]
                    headers = table[0]

                if re.match(roman_numbers_regex, headers[0]):
                    for i, row in enumerate(table):
                        roman_match = re.match(roman_numbers_regex, row[0])
                        if not roman_match:
                            raise RuntimeError(Fore.RED + f'format error in {i} row, {page.page_number} page')

                        first_roman = roman_to_int(roman_match.group(1))
                        if roman_match.group(3):
                            second_roman = roman_to_int(roman_match.group(3))
                        else:
                            second_roman = first_roman

                        for i in range(first_roman - 1, second_roman):
                            raw_data['Opcode'].append(row[1:])

                elif (len(headers) == 4 
                      and all(len(header) == 4 for header in headers)):

                    #if 32-bit opcode
                    if(len(table) == 2):
                        headers.extend(table[1])
                    raw_data['Opcode'].append(headers)

                elif interrupted_flag:

                    raw_data['SREG'].append(headers)

                elif (headers == status_registers):

                    if(len(table) > 1):
                        raw_data['SREG'].append(table[1])
                    else:
                        interrupted_flag = True

        #parse subchapter mnemonic
        title = subchapter_pages[0].extract_text().split('\n')[0]
        match_title = re.match(r'^\s*\d+\.\d+\s+([A-Z]+)\s*(\(([^()]+)\))?.*', title)

        if not match_title:
            raise RuntimeError(f'title is wrong:{title}')

        main_mnemonic: list[str] = [match_title.group(1)]
        if match_title.group(3):
            other_mnemonics: list[str] = match_title.group(3).split(', ')
            main_mnemonic.extend(other_mnemonics)
        mnemonic = AmbiguousFrozen[str](main_mnemonic)
        
        #check raw data
        subchapter_error = check_subchapter_data(raw_data)
        if subchapter_error: 
            print(Fore.RED + f'{'-'*8}{mnemonic}{'-'*8}')
            print(raw_data)
            for line in subchapter_error:
                print(line)

            if not raw_data['Opcode']: #it should be redone to noral function with user input
                raw_data['Opcode'] = AmbiguousList(['?']) 

        #parse subchapter data
        if raw_data['SREG'].is_ambiguous():
            raise RuntimeError(f'chapter {chapter_number}, format error in SREG table: {raw_data['SREG']}')

        for opcode in raw_data['Opcode']:
            subchapter_data.append({'SREG': raw_data['SREG'], 'Opcode': opcode})

        #adding subchapter data
        for chapter_key in chapter_data.keys():
            intersection = chapter_key & mnemonic
            if (intersection and
                any(x not in cpu_versions for x in intersection)):

                subchapter_data.extend(chapter_data[chapter_key])
                mnemonic = mnemonic | chapter_key
                del chapter_data[chapter_key]
                break

        chapter_data[mnemonic] = subchapter_data


    print(Fore.BLUE + f'[PROCESS]: Completes chapter data extracting')
    return chapter_data


def check_subchapter_data(subchapter_data: dict[str, AmbiguousList[list]]) -> list[str]:
    error_message: list[str] = []
    if not subchapter_data:
        error_message.append(Fore.RED + f'Subchapter data is empty')
    else:
        needed_info = ['Opcode',
                       'SREG',
                        ]
        for e in needed_info:
            if not subchapter_data[e]:
                error_message.append(Fore.RED + f'Chapter data has no {e} section')

    return error_message


def merge_instructions_data(tables_data: PartialInstructionsData, chapter_data: PartialInstructionsData) -> RawInstructionsData:
    merged_data: RawInstructionsData = RawInstructionsData()

    for mnemonics, data in chapter_data.items():
        merged_data[mnemonics] = AmbiguousDict()
        merged_data[mnemonics]['Chapter'] = data

    #key is AmbiguousFrozen
    for mnemonics, data in merged_data.items():
        #table_key is AmbiguousFrozen but has only one element
        for table_mnemonic, table_data in tables_data.items():
            if table_mnemonic & mnemonics:
                if 'Table' in data:
                    data['Table'].extend(table_data)
                else:
                    data['Table'] = table_data

    return merged_data


def roman_to_int(roman):
    if roman is None:
        return 0

    roman_dict = {
        'i': 1,
        'v': 5,
        'x': 10, #I assume that it is enough.
    }

    result = 0

    for i in range(len(roman) - 1): #we will add last in the end

        if (roman_dict[roman[i]] >= roman_dict[roman[i + 1]]):
            result += roman_dict[roman[i]]
        else:
            result -= roman_dict[roman[i]]

    result += roman_dict[roman[-1]]

    return result


def print_data_map(data_map: dict, mess: str = 'data_map') -> None:
    print(Fore.BLUE + f'{'-'*8}{mess}{'-'*8}')
    print(Fore.BLUE + f'size: {len(data_map)}')
    for key, value in data_map.items():
        print(f'{key}: {value}')


def process_tables_extraction(pdf_path: str, dump_dir: str, reload: bool) -> PartialInstructionsData:
    tables_data: PartialInstructionsData
    tables_data_path = dump_dir + '/extracted_tables_data.pkl'
    if reload or not os.path.exists(tables_data_path):
        tables_first_page = 17
        tables_last_page = 22
        needed_tables = extract_needed_tables(pdf_path, tables_first_page, tables_last_page)
        tables_data = extract_tables_data(needed_tables)
        with open(tables_data_path, 'wb') as data_file:
            pickle.dump(tables_data, data_file)
    else:
        with open(tables_data_path, 'rb') as data_file:
            tables_data = pickle.load(data_file)

    return tables_data


def process_chapter_extraction(pdf_path: str, dump_dir: str, reload: bool) -> PartialInstructionsData:
    chapter_data: PartialInstructionsData
    chapter_data_path = dump_dir + '/extracted_chapters_data.pkl'
    if reload or not os.path.exists(chapter_data_path):
        chapter_first_page = 23
        chapter_last_page = 146
        chapter_data = extract_chapter_data(pdf_path, 5, chapter_first_page, chapter_last_page)
        with open(chapter_data_path, 'wb') as data_file:
            pickle.dump(chapter_data, data_file)
    else:
        with open(chapter_data_path, 'rb') as data_file:
            chapter_data = pickle.load(data_file)

    return chapter_data


def parse_cpu_version(unprocessed_instructions_data: RawInstructionsData):
    parsed_instructions_data: RawInstructionsData = RawInstructionsData()

    for mnemonics, data in unprocessed_instructions_data.items():
        mnemonics_cpu_versions = AmbiguousFrozen(mnemonics & cpu_versions)
        mnemonics_names = AmbiguousFrozen(mnemonics - cpu_versions)
        if mnemonics_cpu_versions:
            data['Changed versions'] = mnemonics_cpu_versions
        else:
            data['Changed versions'] = AmbiguousList()

        parsed_instructions_data[mnemonics_names] = data

    return parsed_instructions_data


def replace_incorrect_xml_characters(unprocessed_instructions_data: RawInstructionsData): #unused
    incorrect_xml_chars = ('⇔', #it isn't needed
                            )
    for instruction_data in unprocessed_instructions_data.values():

        for chapter_data in instruction_data['Chapter']:
            for sreg in chapter_data['SREG']:
                for i, sreg_str in enumerate(sreg):
                    sreg[i] = ''.join(['*' if char in incorrect_xml_chars else char for char in sreg_str])

        for table_data in instruction_data['Table']:
            instruction_description: str = table_data['Description']
            table_data['Description'] = instruction_description.replace('\n', ' ')


def name_instruction_data(unprocessed_instructions_data: RawInstructionsData) -> ProcessedInstructionsData:
    named_instructions_data: ProcessedInstructionsData = OrderedDict()

    for mnemonics in unprocessed_instructions_data.keys():
        sorted_mnemonics = sorted(list(mnemonics)) 
        instruction_name = str(sorted_mnemonics[0])

        new_instruction_data = AmbiguousDict() #naming DEAD
        new_instruction_data['Mnemonics'] = mnemonics
        new_instruction_data.update(unprocessed_instructions_data[mnemonics])

        named_instructions_data[instruction_name] = new_instruction_data

    named_instructions_data = OrderedDict(sorted(named_instructions_data.items()))
    return named_instructions_data


def process_instructions_data(unprocessed_instructions_data: RawInstructionsData) -> ProcessedInstructionsData:
    unprocessed_instructions_data = parse_cpu_version(unprocessed_instructions_data)
    # replace_incorrect_xml_characters(unprocessed_ienstructions_data)

    processed_instruction_data = name_instruction_data(unprocessed_instructions_data)
    return processed_instruction_data


