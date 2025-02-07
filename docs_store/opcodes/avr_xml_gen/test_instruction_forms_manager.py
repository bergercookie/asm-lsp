from core.logging import setup_logging
setup_logging()

from core.stages.data_analysis import InstructionFormsManager, InstructionParser
from core.data_processing import ExtractedInstructionsData, AmbiguousDict
import pickle

data_path = './intermediate_data/avr_instructions_data.pkl'
instructions_data: ExtractedInstructionsData

with open(data_path, 'rb') as data:
    instructions_data = pickle.load(data)

storage_location = []
instruction_name = 'LD'
instruction_data: AmbiguousDict = instructions_data[instruction_name]

manager = InstructionFormsManager(storage_location, instruction_name, instruction_data)


def test_request_validation_with_correct_keys():
    assert manager.is_request_valid({'type': 'add', 'target': 'form'}) == True
    assert manager.is_request_valid({'type': 'get', 'target': 'aspects', 'aspects': ['version', 'chapter_data']}) == True
    # assert manager.is_request_valid({'type': 'record', 'target': 'form', 'aspects': {'mnemonics': None, 'table_data': None}, 'form_ind': 0}) == True


def test_request_validation_with_incorrect_keys():
    assert manager.is_request_valid({'not_type': 'add', 'target': 'form'}) == False
    assert manager.is_request_valid({'type': 'add', 'not target': 'form'}) == False


def test_request_validation_with_missed_keys():
    assert manager.is_request_valid({'type': 'add'}) == False
    assert manager.is_request_valid({'target': 'form'}) == False
    assert manager.is_request_valid({'type': 'delete', 'target': 'form'}) == False
    # assert manager.is_request_valid({'type': 'record', 'target': 'form', 'form_ind': 0}) == False
    assert manager.is_request_valid({'type': 'record', 'target': 'form', 'aspects': {'mnemonics': None}}) == False


parser = manager._instruction_parser

def test_request_add():
    forms = manager._forms
    old_forms_len = len(forms)
    manager.request({'type': 'add', 'target': 'form'})
    assert len(forms) == old_forms_len + 1
    manager.request({'type': 'add', 'target': 'form', 'aspects': {'mnemonic': parser.get_aspect_data('mnemonic')}})
    assert len(forms) == old_forms_len + 2

    expected_form = InstructionParser.create_empty_form()
    assert forms[0] == expected_form

    expected_form['mnemonic'] = parser.get_aspect_data('mnemonic')
    assert forms[1] == expected_form


def test_request_delete():
    forms = manager._forms
    forms.clear()
    for i in range(2):
        forms.append(parser.create_empty_form())

    manager.request({'type': 'delete', 'target': 'form', 'form_ind': 0})
    assert len(forms) == 1
    manager.request({'type': 'delete', 'target': 'form', 'form_ind': 0})
    assert len(forms) == 0
    manager.request({'type': 'delete', 'target': 'form', 'form_ind': 0})
    assert len(forms) == 0


def test_request_record():
    if len(manager._forms) < 1:
        manager._forms.append(parser.create_empty_form())
    form = manager._forms[0]
    manager.request({'type': 'record', 'target': 'form', 'form_ind': 0, 'aspects': {'mnemonic': parser.get_aspect_data('mnemonic')}})
    expected_form = InstructionParser.create_empty_form()
    expected_form['mnemonic'] = parser.get_aspect_data('mnemonic')
    assert form == expected_form


def test_request_get():
    if len(manager._forms) < 1:
        manager._forms.append(parser.create_empty_form())
    form = manager.request({'type': 'get', 'target': 'form', 'form_ind': 0})
    expected_form = manager._forms[0]
    assert form == expected_form
