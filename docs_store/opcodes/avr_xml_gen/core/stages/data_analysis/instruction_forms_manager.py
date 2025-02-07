from collections.abc import Iterable
import re

from numpy import isin, require
from core.data_processing import InstructionForm, DataManager
from core.data_processing.ambiguous_data import AmbiguousDict
from core.stages.xml_generation import InstructionXMLBuilder
from typing import Any
import copy
import logging


logger = logging.getLogger(__name__)


class InstructionParser:
    def __init__(self, instruction_name: str, instruction_data: AmbiguousDict):
        self._set_instruction(instruction_name, instruction_data)


    def set_instruction(self, instruction_name: str, instruction_data: AmbiguousDict | None = None):
        if instruction_data is None:
            instruction_data = self._instruction_data
        self._set_instruction(instruction_name, instruction_data)


    def _set_instruction(self, instruction_name: str, instruction_data: AmbiguousDict):
        self._instruction_name = instruction_name
        self._instruction_data = instruction_data


    @classmethod
    def create_empty_form(cls) -> InstructionForm:
        form = InstructionForm()
        for aspect_name in InstructionForm.valid_keys:
            form[aspect_name] = None
        return form


    def get_aspects(self) -> dict:
        aspects = {}
        for aspect_name in InstructionForm.valid_keys:
            aspects[aspect_name] = self.get_aspect_data(aspect_name)
        return aspects


    def get_aspect_data(self, aspect_name: str) -> list:
        match aspect_name:
            case 'mnemonic':
                return self._extract_mnemonics()
            case 'version':
                return self._extract_versions()
            case 'table_data':
                return self._extract_table_data()
            case 'chapter_data':
                return self._extract_chapter_data()
            case _:
                logger.error('Incorrect aspect name')
                return []


    def get_name(self) -> str:
        return self._instruction_name


    def _extract_mnemonics(self) -> list:
        mnemonics_list = list(self._instruction_data['Mnemonics'])
        mnemonics_list = sorted(mnemonics_list)
        return mnemonics_list


    def _extract_versions(self) -> list:
        versions = self._instruction_data['Changed versions']
        versions_list = []
        if isinstance(versions, Iterable):
            versions_list = list(versions)
            versions_list = sorted(versions_list)
        versions_list.append('All')
        return versions_list


    def _extract_table_data(self) -> list:
        return list(self._instruction_data['Table'])


    def _extract_chapter_data(self) -> list:
        return list(self._instruction_data['Chapter'])


    def is_instruction_ambiguous(self) -> bool:
        return self._instruction_data.is_ambiguous()



class InstructionFormsManager(DataManager):
    # One instruction could have several forms.
    #
    # Request looks like {'type': ..., 'target': ..., ...}
    # There are two mandatory keys: 'type' and 'target'.
    #
    # 'type' described in parent class (DataManager).
    #
    # Valid 'target' values are in valided_request_keys.
    # There are two options for target:
    # 1. Instruction form data ('form')
    # 2. Aspects data ('aspects')
    #
    # Instruciton form data relates to the particular form and its aspects.
    # Request needs the following:*
    # 1. 'type': ...
    # 2. 'target': 'form'
    # [3]. 'form_ind': any integer within forms list lentgh if type is record, get, delete
    # [4]. 'aspects' key is needed for record, add and optional for get
    # 4.1. type is record:
    #      'aspects': {aspect_name: aspect_data, ...}
    #      aspects item specifies which aspects of the form to change.
    # [4.2]. type is get:
    #        'aspects': [aspect_name, ...]
    #        aspects item specifies which aspects of the form to get.
    # 4.3. type is add:
    #      'aspects': {aspect_name: aspect_data, ...}
    #      aspects item is optional. It specifies which aspects to change in new form.
    #
    # Aspects data relates to the all possible aspects value.
    # Aspects data is independent of the instruction forms.
    # Request needs the following:
    # 1. 'type': 'get' is implemented only for get. I didn't found sencec in other request types
    # 2. 'target': 'aspects'
    # 3. 'aspects': [aspect_name, ...]
    #
    #*if point in [], it could be optional. Look at [3] in Instruction form request needs

    valid_request_keys = {'type': ('record',
                                   'get',
                                   'add',
                                   'delete',
                                   ),
                          'target': ('form',
                                     'aspects',
                                     'name', #temp decision
                                     ),
                          #there should be 'aspects', but I don't know how to properly add it here
                          }
    def __init__(self, storage_location: list[InstructionForm], instruction_name: str, instruction_data: AmbiguousDict) -> None:
        self._storage_location = storage_location
        self._instruction_parser = InstructionParser(instruction_name, instruction_data)
        self._forms: list[InstructionForm] = []


    def save(self) -> None:
        self._storage_location.clear()
        self._storage_location.extend(self._forms)


    def is_request_valid(self, request: dict, log=True) -> bool:
        if not self._is_request_key_valided(request, 'type', log):
            return False
        if not self._is_request_key_valided(request, 'target', log):
            return False
        match request['type']:
            case 'record':
                if request['target'] == 'form':
                    if not self._is_key_in_request(request, 'form_ind', log): return False
                    if not self._is_form_ind_valid(request, log): return False
                    if not self._is_key_in_request(request, 'aspects', log): return False
                    if not self._is_aspects_dict_valid(request, log): return False
                elif request['target'] == 'aspects':
                    if log: logger.error(f'Invalid request {request}; type:record for target:aspect isn\'t implemented')
                    return False

            case 'get':
                if request['target'] == 'form':
                    if not self._is_key_in_request(request, 'form_ind', log): return False
                    if not self._is_form_ind_valid(request, log): return False
                    if self._is_key_in_request(request, 'aspects', False):
                        if not self._is_aspects_list_valid(request, log): return False
                elif request['target'] == 'aspects':
                    if self._is_key_in_request(request, 'aspects', False):
                        if not self._is_aspects_list_valid(request, log): return False

            case 'add':
                if request['target'] == 'form':
                    if self._is_key_in_request(request, 'aspects', False):
                        if not self._is_aspects_dict_valid(request, log): return False
                elif request['target'] == 'aspects':
                    if log: logger.error(f'Invalid request {request}; type:add for target:aspect isn\'t implemented')
                    return False

            case 'delete':
                if request['target'] == 'form':
                    if not self._is_key_in_request(request, 'form_ind', log): return False
                    if not self._is_form_ind_valid(request, log): return False
                elif request['target'] == 'aspects':
                    if log: logger.error(f'Invalid request {request}; type:delete for target:aspect isn\'t implemented')
                    return False
        return True


    def _is_aspects_list_valid(self, request, log=True) -> bool:
        if not isinstance(request['aspects'], list):
            if log: logger.error(f'Invalid request {request}; aspects should be a list')
            return False
        valid_keys = set(InstructionForm.valid_keys)
        wrong_keys = set(request['aspects']) - valid_keys
        if wrong_keys:
            if log: logger.error(f'Invalid request {request}; aspects list have unexpected values {wrong_keys}')
            return False
        return True


    def _is_aspects_dict_valid(self, request, log=True) -> bool:
        if not isinstance(request['aspects'], dict):
            if log: logger.error(f'Invalid request {request}; aspects should be a dict')
            return False
        valid_keys = set(InstructionForm.valid_keys)
        wrong_keys = set(request['aspects'].keys()) - valid_keys
        if wrong_keys:
            if log: logger.error(f'Invalid request {request}; aspects dict have unexpected keys {wrong_keys}')
            return False
        return True


    def _is_form_ind_valid(self, request, log=True) -> bool:
        form_ind = request['form_ind']
        if form_ind == 'all':
            return True
        if not isinstance(form_ind, int):
            if log: logger.error(f'Invalid request {request}; form_ind should be int or \'all\'')
            return False
        if not (0 <= form_ind and form_ind < len(self._forms)):
            if log: logger.error(f'Invalid request {request}; form_ind out of range')
            return False
        return True


    def request(self, request: dict) -> Any:
        if not self.is_request_valid(request):
            return
        match request['type']:
            case 'record':
                self._record_request(request)
            case 'get':
                return self._get_request(request)
            case 'add':
                self._add_request(request)
            case 'delete':
                self._delete_request(request)


    def _record_request(self, request: dict) -> None:
        if request['target'] == 'form':
            if request['form_ind'] == 'all':
                for form in self._forms:
                    form.update(request['aspects'])
            else:
                form = self._forms[request['form_ind']]
                form.update(request['aspects'])


    def _get_request(self, request) -> str | list | dict | InstructionForm:
        response: str | list | dict | InstructionForm
        match request['target']:
            case 'form':
                if request['form_ind'] == 'all':
                    response = []
                    if 'aspects' in request:
                        for form in self._forms:
                            form_aspects = {}
                            for aspect_name in request['aspects']:
                                form_aspects[aspect_name] = form[aspect_name]
                            response.append(form_aspects)
                    else:
                        response = copy.deepcopy(self._forms)
                else:
                    form = self._forms[request['form_ind']]
                    if 'aspects' in request:
                        response = {}
                        for aspect_name in request['aspects']:
                            response[aspect_name] = form[aspect_name]
                    else:
                        response = copy.deepcopy(form)
            case 'aspects':
                response = {}
                if 'aspects' in request:
                    for aspect_name in request['aspects']:
                        response[aspect_name] = self._instruction_parser.get_aspect_data(aspect_name)
                else:
                    response = copy.deepcopy(self._instruction_parser.get_aspects())
            case 'name':
                response = self._instruction_parser.get_name()
            case _:
                raise RuntimeError('request validation is wrong: request parsing get unvalid values')
        return response


    def _add_request(self, request) -> None:
        if request['target'] == 'form':
            new_form = self._instruction_parser.create_empty_form()
            if 'aspects' in request:
                new_form.update(request['aspects'])
            self._forms.append(new_form)


    def _delete_request(self, request) -> None:
        if request['target'] == 'form':
            self._forms.pop(request['form_ind'])


    def is_instruction_ambiguous(self) -> bool:
        return self._instruction_parser.is_instruction_ambiguous()
