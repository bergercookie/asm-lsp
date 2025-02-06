#internal
from .stage import BidirectionalStage, StageGUI, StageTask

#external
from ..data_processing import InstructionDataMarkers, Context, DataManager, SourceInfo
from ..gui import BackNextButtons, PDFRegionSelector
import tkinter as tk


#There should be several Stages related to the data markers with common DataManagaer
#But in reality I didn't use data markers
#I decided to create example of using Stage (only one: Settingheader)

class DataMarkersManager(DataManager):
    #request is structured like {'type': set_height, 'elements': {...}}
    #could look like overkill, but check InstructionFromsManager in core/stages/data_analysis
    #this class is just a plug for demostratoin
    valid_request_keys = {'type': ('set_height', 'get_height')}
    expected_elements = {'word', 'line', 'curve'}

    def __init__(self, instruction_data_markers: InstructionDataMarkers) -> None:
        self._instruction_data_markers = instruction_data_markers

    def is_request_valid(self, request: dict) -> bool:
        if not self._is_request_key_valided(request, 'type'):
            return False
        match request['type']:
            case 'set_height':
                return self.expected_elements == request['elements'].keys()
            case 'get_height':
                return True
        return True

    def __get_max_height(self, elements: dict) -> int:
        #there is 0 because I have no idea about using negative height in pdf
        max_height = 0
        for group in elements.values():
            for e in group:
                if max_height < e['bottom']:
                    max_height = e['bottom']
        return max_height

    def request(self, request: dict) -> None | object:
        if not self.is_request_valid(request):
            return
        match request['type']:
            case 'set_height':
                self._instruction_data_markers['header_height'] = self.__get_max_height(request['elements'])
            case 'get_height':
                return self._instruction_data_markers['header_height']


class SettingHeaderTask(StageTask):
    def __init__(self, context: Context) -> None:
        super().__init__(context)
        #there is IDE error due to the declaraion of get_data
        #I vould like to point that get_data(data) returns type(data), but syntax f(v) -> g(v): pass incorrect
        #This nuance will pop in all get_data() calls
        self._instruction_data_markers: InstructionDataMarkers = self._context[InstructionDataMarkers]

    def is_completed(self) -> bool:
        header_height = self._instruction_data_markers['header_height']
        if isinstance(header_height, (int, float)):
            return True
        return False


class SettingHeaderGUI(StageGUI):
    def __init__(self, data_manager: DataMarkersManager, master=None, cnf={}, **kwargs) -> None:
        super().__init__(data_manager, master, cnf, **kwargs)
        self.__setup_ui()
        self.__setup_bindings()

    def __setup_ui(self) -> None:
        self._pdf_region_selector = PDFRegionSelector(self, pdf_selection=True)
        self._info_label = tk.Label(self, text='Choose an upper bound')
        self._back_next_buttons = BackNextButtons(self, epoint=self)
        self._pdf_region_selector.pack(side=tk.TOP, fill=tk.BOTH, expand=True)
        self._info_label.pack(side=tk.TOP, fill=tk.X)
        self._back_next_buttons.pack(side=tk.TOP, fill=tk.X)

    def __setup_bindings(self) -> None:
        self._pdf_region_selector.bind('<<Select>>', self.__on_highlight, add='+')

    def __on_highlight(self, event) -> None:
        if self._data_manager is None:
            raise RuntimeError('error in programm logic: expected non None data manager')
        self._data_manager.request({'type': 'set_height', 'elements': self._pdf_region_selector.get_selected_elements()})
        height = self._data_manager.request({'type': 'get_height'})
        self._info_label.configure(text=f'Upper bound height: {height}')

    def set_pdf(self, pdf_path: str) -> None:
        self._pdf_region_selector.set_pdf(pdf_path)


class SettingHeader(BidirectionalStage):
    def __init__(self, context: Context, master=None, cnf={}, **kwargs) -> None:
        super().__init__(context, master, cnf, **kwargs)
        #The same as in SettingHeaderTask.__init__()
        self._data_manager = DataMarkersManager(self._context[InstructionDataMarkers])
        self.set_gui(SettingHeaderGUI(self._data_manager, master, cnf,**kwargs))
        self.set_task(SettingHeaderTask(self._context))

    def execute(self) -> None:
        super().execute()
        #The same as in SettingHeaderTask __init__()
        pdf_path = self._context[SourceInfo]['pdf_path']
        #I don't know how to fix warnings '_gui has no set_pdf function
        #It is raised because _gui is declared as StageGUI type in parent class
        self._gui.set_pdf(pdf_path)

