# internal
from .data_extraction_manager import DataExtractionManager

# external
from core.stages import Stage, StageGUI, StageTask
from core.data_processing.data_management import DataManager, Context
from core.gui import NextButton
import tkinter as tk

# It has no sence, but I am tired to make it seriously
class DataExtractionTask(StageTask):
    def is_completed(self) -> bool:
        return True


class DataExtractionGUI(StageGUI):
    def __init__(self, data_manager: DataManager, master=None, cnf=..., **kwargs):
        super().__init__(data_manager, master, cnf, **kwargs)
        self.__setup_ui()

    def __setup_ui(self):
        self._label = tk.Label(self, text='Please, vait')
        self._next_button = NextButton(self, epoint=self, state=tk.DISABLED)
        self._label.pack(fill=tk.BOTH, expand=True)
        self._next_button.pack(side=tk.BOTTOM, fill=tk.X)

    def set_completed(self):
        self._label.config(text='Extraction is completed')
        self._next_button.config(state=tk.NORMAL)


class DataExtraction(Stage):
    def __init__(self, context: Context, master=None, cnf=..., **kwargs):
        super().__init__(context, master, cnf, **kwargs)
        self._data_manager = DataExtractionManager(self._context)
        self.set_gui(DataExtractionGUI(self._data_manager, master, cnf, **kwargs))
        self.set_task(DataExtractionTask(self._context))

    def execute(self):
        super().execute()
        self._data_manager.request({'type': 'start'})
        # This error is described in data_markers.py Settingheader.execute()
        self._gui.set_completed()
        self.permanently_completed = True
