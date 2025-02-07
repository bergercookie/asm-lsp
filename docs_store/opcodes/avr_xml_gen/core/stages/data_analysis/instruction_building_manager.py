# internal
from .instruction_forms_manager import InstructionFormsManager

# external
from core.data_processing.ambiguous_data import AmbiguousDict
from core.data_processing.data_management import InstructionForm, Subject
from typing import Any
import logging


logger = logging.getLogger(__name__)


class InstructionBuildingManager(InstructionFormsManager, Subject):
    def __init__(self, storage_location: list[InstructionForm], instruction_name: str, instruction_data: AmbiguousDict) -> None:
        InstructionFormsManager.__init__(self, storage_location, instruction_name, instruction_data)
        Subject.__init__(self)
        self.focused_form_ind: int | None = None


    # it should me created with reqeuest({'type: 'record', 'target': 'focused_ind', 'value': ind})
    # but it looks like overkill, so I should think over requests in instrucion_forms_manager
    def focus_form(self, ind: int | None) -> None:
        self.notify('before_focused_form_changed')
        self.focused_form_ind = ind
        self.notify('after_focused_form_changed')


    def is_request_valid(self, request: dict, log=True) -> bool:
        if ('form_ind' in request and
            request['form_ind'] == 'focused'
            ):
            if self.focused_form_ind is None:
                if log: logger.debug(f'Request {request} failed; focused form absents')
                return False
            else:
                request['form_ind'] = self.focused_form_ind
        return super().is_request_valid(request, log)


    def request(self, request) -> Any:
        response = super().request(request)
        # notify about form-shifitng events
        match request['type']:
            case 'record':
                self.notify('record')
            case 'add':
                self.notify('add')
            case 'delete':
                self.notify('delete')
        # changing self.focused_form_ind after deleting
        if (request['type'] == 'delete' and
            request['type'] == 'form' and
            request['form_ind'] == self.focused_form_ind
            ):
            if (self.focused_form_ind is not None and
                self.focused_form_ind >= len(self._forms) # if out of _forms. After 'delete' len = len - 1
                ):
                if len(self._forms) > 0:
                    self.focused_form_ind -= 1
                else:
                    self.focused_form_ind = None
        return response
