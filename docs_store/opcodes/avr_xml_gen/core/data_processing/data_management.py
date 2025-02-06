from abc import ABC, abstractmethod
from typing import OrderedDict, Any
import logging
from .ambiguous_data import AmbiguousDict
import weakref

logger=logging.getLogger(__name__)

#Dictionaty wich could have only particular keys
#This keys are placed in valid_keys
#Class guarantees setting all keys from valid_keys even if they weren't passed in __init__
#Also it has unique id. It is used for core/stages/data_analysis/instruction_building_manager
class StrictDictionary(dict):
    valid_keys = ()

    def __init__(self, *args, **kwargs):
        for k in kwargs.keys():
            if not self.is_valid_aspect_name(k):
                return
        super().__init__(*args, **kwargs)
        for k in self.valid_keys:
            if k not in self.keys():
                self[k] = None
        self._id = hash(id(self))

    def id(self):
        return self._id

    def update(self, *args, **kwargs):
        for k in kwargs.keys():
            if not self.is_valid_aspect_name(k):
                return
        super().update(*args, **kwargs)

    def __setitem__(self, k, v):
        if not self.is_valid_aspect_name(k):
            return
        super().__setitem__(k, v)

    def is_valid_aspect_name(self, k: str) -> bool:
        if k not in self.valid_keys:
            logger.error(f'Try to set unvalid key: {k}')
            return False
        return True



class SourceInfo(StrictDictionary):
    valid_keys = ('pdf_path',
                  )

class InstructionDataMarkers(StrictDictionary):
    valid_keys = ('header_height',
                  )

class InstructionForm(StrictDictionary):
    valid_keys = ('mnemonic',
                  'version',
                  'table_data',
                  'chapter_data',
                  )

class ExtractedInstructionsData(OrderedDict[str, AmbiguousDict]):
    pass

class ProcessedInstructionsData(OrderedDict[str, list[InstructionForm]]):
    pass


class DataManager(ABC):
    #Request is dictionary
    #By default 'type':... is mandaotry item of request
    #Valid values for 'type' are in valid_request_keys
    valid_request_keys: dict = {'type': ()}

    #log parameter determines whether to log or not to log
    def _is_key_in_request(self, request: dict, request_key: str, log=True) -> bool:
        if request_key not in request.keys():
            if log: logger.error(f'Invalid request {request}; expected key {request_key}')
            return False
        return True

    #it is named with 'valided' word instead of 'valid' due to the problems wit aspects key in InstructionFormsManager
    #i should change logic someday...
    def _is_request_key_valided(self, request: dict, request_key: str, log=True) -> bool:
        if not self._is_key_in_request(request, request_key, log): 
            return False
        if request[request_key] not in self.valid_request_keys[request_key]:
            if log: logger.error(f'Invalid request {request}; {request_key}: {request[request_key]} is invalid')
            return False
        return True

    @abstractmethod
    def is_request_valid(self, request: dict) -> bool:
        pass

    @abstractmethod
    def request(self, request: dict) -> None | object:
        pass


class Observer(ABC):
    @abstractmethod
    def update(self, message=None):
        pass


class Subject:
    def __init__(self):
        self._observers: list[weakref.ReferenceType[Observer]] = []

    def notify(self, message=None):
        for observer in self._observers:
            observer().update(message)

    def attach(self, observer: Observer):
        if all(observer != wr() for wr in self._observers):
            self._observers.append(weakref.ref(observer))

    def detach(self, observer: Observer):
        if observer in self._observers:
            self._observers.remove(observer)


#Stores objects of any type. Type itself is access key to the object
#e.g. require(int) returns variable with type int
class Context:
    def __init__(self):
        self._data_dict: dict = {}

    def record(self, data): 
        data_type = type(data)
        if data_type in self._data_dict.keys():
            logger.info(f'Data overwriting: {self._data_dict[data_type]} -> {data}')
        self._data_dict[data_type] = data

    #returns value with data_type type. e.g value = require(data_type), type(value) == data_type
    def __getitem__(self, data_type: type) -> Any:
        if not data_type in self._data_dict.keys():
            logger.debug(f'Data acess failed: {data_type}')
            return None
        return self._data_dict[data_type]
