#internal
from ..instruction_building_manager import InstructionBuildingManager

#external
from core.data_processing.data_management import InstructionForm, Observer
from core.gui.common_gui import OptionSelector, InfoDisplay
from core.stages.xml_generation import InstructionXMLBuilder
import tkinter as tk
import logging


logger = logging.getLogger()


class InstructionFormSelector(tk.Frame, Observer):
    def __init__(self, instruction_manager: InstructionBuildingManager, master=None, cnf={}, **kwargs) -> None:
        tk.Frame.__init__(self, master, cnf, **kwargs)
        Observer.__init__(self)
        self._instruction_manager = instruction_manager
        self._instruction_manager.attach(self)
        self.__setup_ui()
        self.__setup_bindings()


    def __setup_ui(self) -> None:
        self._form_selection_area = tk.Frame(self)
        self._form_selector = OptionSelector(self._form_selection_area)
        self._buttons_area = tk.Frame(self._form_selection_area)
        self._add_button = tk.Button(self._buttons_area, text='add')
        self._delete_button = tk.Button(self._buttons_area, text='delete')
        self._form_info = InfoDisplay(self)

        request = {'type': 'get', 'target': 'name'}
        name = self._instruction_manager.request(request)
        self._form_selector.set_name(name + ' forms')
        self._form_info.set_name('Result')

        self._form_selection_area.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        self._form_selector.pack(side=tk.TOP, fill=tk.BOTH, expand=True)
        self._buttons_area.pack(side=tk.BOTTOM, fill=tk.X)
        self._add_button.pack(side=tk.LEFT, fill=tk.X, expand=True)
        self._delete_button.pack(side=tk.LEFT, fill=tk.X, expand=True)
        self._form_info.pack(side=tk.RIGHT, fill=tk.BOTH, expand=True)


    def __setup_bindings(self) -> None:
        #i don't know how to make it properly without accessing to the internal object of aspect_selector
        self._form_selector._scroll_listbox.bind('<Return>', self._select_form, add='+')
        self._form_selector._scroll_listbox.bind('<Double-Button-1>', self._select_form, add='+')
        self._form_selector._scroll_listbox.bind('<<Down>>', self._show_preliminar_info, add='+') 
        self._form_selector._scroll_listbox.bind('<<Up>>', self._show_preliminar_info, add='+')
        self._form_selector._scroll_listbox.bind('<<ListboxSelect>>', self._show_preliminar_info, add='+')
        self._add_button.config(command=self.add_form)
        self._delete_button.config(command=self.delete_form)


    def _select_form(self, event=None) -> None:
        selected_line_ind = self._form_selector._selected_line_ind
        self._instruction_manager.focus_form(selected_line_ind)


    def update(self, message=None):
        if (message is None or
            message in ['record',
                        'add',
                        'delete',
                        ]
            ):
            request = {'type': 'get', 'target': 'form', 'form_ind': 'all'}
            forms = self._instruction_manager.request(request)
            if not isinstance(forms, list):
                raise RuntimeError(f'invalid responce in request {request}; expected an list, not {type(forms)}')
            self._form_selector.delete(0, tk.END)
            for form in forms:
                self._form_selector.insert(tk.END, form)
            self._form_selector.update()
            self._show_preliminar_info()
        elif message == 'after_focused_form_changed':
            self._show_preliminar_info()


    def add_form(self):
        request = {'type': 'add', 'target': 'form'}
        self._instruction_manager.request(request)


    def delete_form(self, form_ind: int | None = None):
        if form_ind is None:
            form_ind = self._form_selector.get_current_ind()
            if form_ind is None:
                if self._form_selector._scroll_listbox.size() > 0:
                    form_ind = self._form_selector._scroll_listbox.size() - 1
                else:
                    return
        request = {'type': 'delete', 'target': 'form', 'form_ind': form_ind}
        self._instruction_manager.request(request)


    def _show_preliminar_info(self, event=None, form_ind: int | None = None):
        if form_ind is None:
            form_ind = self._form_selector.get_current_ind()
            if form_ind is None:
                self._form_info.clear()
                self._form_info.insert(tk.END, 'Please, choose instruction form')
                return
        request = {'type': 'get', 'target': 'form', 'form_ind': form_ind}
        instruction_form = self._instruction_manager.request(request)
        if not isinstance(instruction_form, InstructionForm):
            logger.error(f'Invalid request {request}; expected an InstructionForm, not {type(instruction_form)}')
            return
        instruction_form_str = InstructionXMLBuilder.create_instruction_form_str(instruction_form)
        self._form_info.clear()
        self._form_info.insert(tk.END, instruction_form_str)
