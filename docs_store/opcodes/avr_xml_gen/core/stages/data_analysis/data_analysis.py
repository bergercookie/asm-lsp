# internal
from .gui.instruction_form_selector import InstructionFormSelector
from .gui.form_aspects_selector import FormAspectsSelector
from .instruction_building_manager import InstructionBuildingManager

# external
from core.stages import BidirectionalStage, StageGUI, StageTask
from core.gui.common_gui import BackNextButtons
from core.data_processing.data_management import Context, ExtractedInstructionsData, InstructionForm, ProcessedInstructionsData
from core.data_processing.ambiguous_data import AmbiguousDict
import logging
import tkinter as tk
from tkinter import ttk


logger = logging.getLogger(__name__)


class InstructionBuildingTask(StageTask):
    def __init__(self, instruction_name: str, context: Context) -> None:
        super().__init__(context)
        self._instruction_name = instruction_name

    def is_completed(self) -> bool:
        processed_data: list[InstructionForm] = self._context[ProcessedInstructionsData][self._instruction_name]
        return len(processed_data) > 0


class InstructionBuildingGUI(StageGUI):
    def __init__(self, instruction_manager: InstructionBuildingManager, master=None, cnf={}, **kwargs) -> None:
        super().__init__(instruction_manager, master, cnf, **kwargs)
        self._tab_control = ttk.Notebook(self)
        self._forms = InstructionFormSelector(instruction_manager, self._tab_control)
        self._mnemonic_and_version = FormAspectsSelector(instruction_manager, ['mnemonic', 'version'], self._tab_control)
        self._table_data = FormAspectsSelector(instruction_manager, ['table_data'], self._tab_control)
        self._chapter_data = FormAspectsSelector(instruction_manager, ['chapter_data'], self._tab_control)
        self._tab_control.add(self._forms, text='Instruction forms')
        self._tab_control.add(self._mnemonic_and_version, text='Mnemonic&Version')
        self._tab_control.add(self._table_data, text='Table data')
        self._tab_control.add(self._chapter_data, text='Chapter data')
        instruction_manager.attach(self._forms)
        instruction_manager.attach(self._mnemonic_and_version)
        instruction_manager.attach(self._table_data)
        instruction_manager.attach(self._chapter_data)
        self._back_next_buttons = BackNextButtons(self, epoint=self)
        self._tab_control.pack(side=tk.TOP, fill=tk.BOTH, expand=True)
        self._back_next_buttons.pack(side=tk.BOTTOM, fill=tk.X)


class InstructionBuilding(BidirectionalStage):
    def __init__(self, instruction_name: str, context: Context, master=None, cnf={}, **kwargs) -> None:
        super().__init__(context, cnf, **kwargs)
        self._context[ProcessedInstructionsData][instruction_name] = []
        storage_location: list[InstructionForm] = self._context[ProcessedInstructionsData][instruction_name]
        instruction_data: AmbiguousDict = self._context[ExtractedInstructionsData][instruction_name]
        self._instruction_manager = InstructionBuildingManager(storage_location, instruction_name, instruction_data)
        self.set_gui(InstructionBuildingGUI(self._instruction_manager, master, cnf, **kwargs))
        self.set_task(InstructionBuildingTask(instruction_name, self._context))

    def execute(self) -> None:
        if not self._instruction_manager.is_instruction_ambiguous():
            request = {'type': 'add', 'target': 'form'}
            self._instruction_manager.request(request)
            self._instruction_manager.focus_form(0)
            request = {'type': 'get', 'target': 'aspects'}
            aspects = self._instruction_manager.request(request)
            form_aspects = {}
            for aspect_name, aspect_data in aspects.items():
                form_aspects[aspect_name] = aspect_data[0]
            request = {'type': 'record', 'target': 'form', 'form_ind': 'focused', 'aspects': form_aspects}
            self._instruction_manager.save()
            self.permanently_completed = True
            self._complete()
        else:
            request = {'type': 'add', 'target': 'form'}
            self._instruction_manager.request(request)
            super().execute()

    def try_complete(self, event=None) -> None:
        self._instruction_manager.save()
        return super().try_complete(event)
