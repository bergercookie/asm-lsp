# internal
from core.data_processing.ambiguous_data import AmbiguousList, AmbiguousDict
from ..instruction_building_manager import InstructionBuildingManager

# external
from core.data_processing.data_management import InstructionForm, Observer
from core.gui.common_gui import OptionSelector, InfoDisplay
from core.stages.xml_generation import InstructionXMLBuilder
import tkinter as tk
import logging


logger = logging.getLogger(__name__)


# There are dead naming. For example setup_ui() should be internal method (aka __setup_ui()), but I would like to save overload option.
# I mean, setup_ui() could be overloaded and executed without overloading __init__().
# But __init__() is defined in parent, so if it calls __setup_ui() (not setup_ui()), it always calls parents __setup_ui() implementation,
# instead of child's due to the __ in naming
# I don't know python, so don't realize how should I name this kind of functions (cinternal, but could be overloaded by inheritor)
class FormAspectsSelector(tk.Frame, Observer):
    def __init__(self, instruction_manager: InstructionBuildingManager, aspect_selectors: list[str], master = None, cnf={}, **kwargs):
        tk.Frame.__init__(self, master, cnf={}, **kwargs)
        Observer.__init__(self)
        self._instruction_manager = instruction_manager
        # aspect could be opcode, SREG (status registers), flags, ...
        # aspect option - one of the possible aspect values
        # e.g. SREG could be [1, 0, -, -] or [0, 0, 1, 1] or ...
        self._aspect_selectors: dict[str, OptionSelector] = {}
        # each option has str name in listbox and it's value
        # aspect option is displayed as str in listbox line. But aspect could be
        # complex data type, so for each aspect option line we need to store
        # aspect option value. Option line is list ind, option value is list[ind]
        self._metadata: dict[str, list] = {} # dict[aspect selector id, list]
        # program saves shoosen aspect opitons for each instruction form
        self._form_snapshots: dict[int, dict[str, int | None]] = {} # dict[form id, dict[aspect name, int | None]
        self.__setup_ui(aspect_selectors)
        self.__setup_bindings()


    def __setup_ui(self, aspect_selectors_names: list[str]):
        # init aspect selectors
        self._aspect_selectors_area = tk.Frame(self)
        for aspect_name in aspect_selectors_names:
            # option_selector and aspect_selector are the same
            option_selector = OptionSelector(self._aspect_selectors_area)
            option_selector.set_name('Aspect: ' + aspect_name)
            self._aspect_selectors[aspect_name] = option_selector
            self._metadata[aspect_name] = []
        # fill aspect selectors options
        request = {'type': 'get', 'target': 'aspects', 'aspects': aspect_selectors_names}
        aspects_options: dict[str, list] = self._instruction_manager.request(request)
        for aspect_name, aspect_options in aspects_options.items():
            for aspect_option in aspect_options:
                # option_selector and aspect_selector are the same
                aspect_selector = self._aspect_selectors[aspect_name]
                aspect_selector.insert(tk.END, aspect_option)
                self._metadata[aspect_name].append(aspect_option)
        # init info display
        self._info_display = InfoDisplay(self)
        self._info_display.set_name('Result')
        self._show_preliminar_info()
        # pack all
        for aspect_selector in self._aspect_selectors.values():
            aspect_selector.pack(side=tk.TOP, fill=tk.BOTH, expand=True)
        self._aspect_selectors_area.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        self._info_display.pack(side=tk.RIGHT, fill=tk.BOTH, expand=True)


    def __setup_bindings(self):
        for aspect_name, aspect_selector in self._aspect_selectors.items():
            if not aspect_selector:
                raise RuntimeError('Aspect is empty')
            # i don't know how to make it properly without accessing to the internal object of aspect_selector
            aspect_selector._scroll_listbox.bind('<Return>', lambda event, name=aspect_name: self._select_aspect_option(name), add='+')
            aspect_selector._scroll_listbox.bind('<Double-Button-1>', lambda event, name=aspect_name: self._select_aspect_option(name), add='+')
            aspect_selector._scroll_listbox.bind('<<Down>>', lambda event, name=aspect_name: self._show_preliminar_info(name), add='+')
            aspect_selector._scroll_listbox.bind('<<Up>>', lambda event, name=aspect_name: self._show_preliminar_info(name), add='+')
            aspect_selector._scroll_listbox.bind('<<ListboxSelect>>', lambda event, name=aspect_name: self._show_preliminar_info(name), add='+')


    def _snapshot_form(self, form: InstructionForm):
        form_id = form.id()
        if not form_id in self._aspect_selectors:
            self._form_snapshots[form_id] = {}
        for aspect_name, aspect_selector in self._aspect_selectors.items():
            # It is awfull code because instruction_form calls id() method for getting unique id (form_id)
            # But we get aspect_selector by it's name. It means that we point to objects in different ways
            # (point to concrete form by form_id, but point to concrete to aspect by it's str name, not id)
            # I added id() method to the instution_form becasue request() returns copy of instruction_form object
            # So python's id() function returns different value for different copies, but I need them have the same id
            self._form_snapshots[form_id][aspect_name] = aspect_selector.get_selected_ind()


    def _show_preliminar_info(self, aspect_name=None):
        request = {'type': 'get', 'target': 'form', 'form_ind': 'focused'}
        instruction_form = self._instruction_manager.request(request)

        # empty info
        if instruction_form is None:
            self._info_display.clear()
            self._info_display.insert(tk.END, 'Please, choose instruction form')
            return
        # wrong logic
        elif not isinstance(instruction_form, InstructionForm):
            logger.error(f'Invalid request {request}; expected an InstructionForm, not {type(instruction_form)}')
            return

        if aspect_name is not None:
            aspect_option_ind = self._aspect_selectors[aspect_name].get_current_ind()
            if aspect_option_ind is None:
                instruction_form[aspect_name] = None
            else:
                instruction_form[aspect_name] = self._metadata[aspect_name][aspect_option_ind]
        instruction_form_str = InstructionXMLBuilder.create_instruction_form_str(instruction_form)
        self._info_display.clear()
        self._info_display.insert(tk.END, instruction_form_str)


    def _select_aspect_option(self, aspect_name: str):
        aspect_option_ind = self._aspect_selectors[aspect_name].get_selected_ind()
        if aspect_option_ind is None:
            aspect_option = None
        else:
            aspect_option = self._metadata[aspect_name][aspect_option_ind]
        request = {'type': 'record', 'target': 'form', 'form_ind': 'focused', 'aspects': {aspect_name: aspect_option}}
        self._instruction_manager.request(request)


    # here is shitcode
    def update(self, message=None):
        # shitcode
        if message in ('record',
                           'add',
                           'delete',
                           'before_focused_form_changed',
                           'after_focused_form_changed'
                           ):
            # it is used in all cases, but message could be beyond cases
            # so it requests each update. It isn't good, because it doesn't need do it
            request = {'type': 'get', 'target': 'form', 'form_ind': 'focused'}
            focused_form = self._instruction_manager.request(request)
            if focused_form is None:
                # clear aspect selector
                for aspect_selector in self._aspect_selectors.values():
                    aspect_selector._selected_line_ind = None
            elif not isinstance(focused_form, InstructionForm):
                logger.error(f'Invalid request {request}; expected an InstructionForm, not {type(focused_form)}')
                # clear aspect selector
                for aspect_selector in self._aspect_selectors.values():
                    aspect_selector._selected_line_ind = None
            else:
                # see about focused_form.id() in _snapshot_form function
                focused_form_id = focused_form.id()
                match message:
                    case 'before_focused_form_changed':
                        logger.debug(f'Before focused form changed. Focused form id: {focused_form_id}')
                        self._snapshot_form(focused_form)
                    case 'after_focused_form_changed':
                        logger.debug(f'After forcused form changed. Focused form id: {focused_form_id}')
                        # it isn't snapshot. It is just setting start position on foucsed form
                        if not focused_form_id in self._form_snapshots:
                            for aspect_name, aspect_selector in self._aspect_selectors.items():
                                if aspect_selector._scroll_listbox.size() == 1:
                                    aspect_selector._selected_line_ind = 0
                                else:
                                    aspect_selector._selected_line_ind = None
                                self._select_aspect_option(aspect_name)
                        else:
                            for aspect_name, aspect_selector in self._aspect_selectors.items():
                                aspect_selector._selected_line_ind = self._form_snapshots[focused_form_id][aspect_name]
                                self._select_aspect_option(aspect_name)
                    # it should be form_delete, but program is too simple
                    # and it sucks because even after failed reqest, notify sends 'delete'.
                    # see instruction_building_manager implementation
                    case 'delete':
                        del self._form_snapshots[focused_form_id]

        for aspect_name, aspect_selector in self._aspect_selectors.items():
            aspect_selector.update()
            self._show_preliminar_info(aspect_name)
