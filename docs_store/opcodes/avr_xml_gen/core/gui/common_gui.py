import tkinter as tk
from typing import Any
import logging

logger = logging.getLogger(__name__)

class ScrollListbox(tk.Listbox):
    def __init__(self, master=None, cnf={}, **kwargs):
        # super().__init__(master, cnf, **kwargs)
        #make some trick
        self._frame = tk.Frame(master)
        super().__init__(self._frame, cnf, selectmode=tk.SINGLE, **kwargs)
        super().pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        #setup scrollbar
        self._vscrollbar = tk.Scrollbar(self._frame, orient=tk.VERTICAL, command=self.yview)
        self._vscrollbar.pack(side=tk.RIGHT, fill=tk.Y)
        super().config(yscrollcommand=self._vscrollbar.set)
        #call custom events
        self.bind('<KeyPress-j>', lambda event: self.event_generate('<<Down>>'), add='+')
        self.bind('<Down>', lambda event: self.event_generate('<<Down>>'), add='+')
        self.bind('<KeyPress-k>', lambda event: self.event_generate('<<Up>>'), add='+')
        self.bind('<Up>', lambda event: self.event_generate('<<Up>>'), add='+')
        #init default events
        self.bind('<<ListboxSelect>>', self.__on_item_click, add='+')
        self.bind('<<Down>>', self.__select_next, add='+')
        self.bind('<<Up>>', self.__select_previous, add='+')
        #init ind
        self.current_ind = None

    #for correct work
    def pack_configure(self, cnf={}, **kwargs):
        self._frame.pack_configure(cnf, **kwargs)

    pack = configure = config = pack_configure

    def pack_forget(self):
        self._frame.pack_forget()

    forget = pack_forget

    def pack_info(self):
        return self._frame.pack_info()


    info = pack_info
    #magic is complete

    def __on_item_click(self, event):
        selection = event.widget.curselection()
        if selection and (selection[0] is not None):
            self.current_ind = selection[0]
            self.select_line(self.current_ind)

    def __select_next(self, event):
        if self.current_ind is None:
            self.current_ind = 0 
        else:
            self.current_ind = (self.current_ind + 1) % self.size()
        self.select_line(self.current_ind)

    def __select_previous(self, event):
        if self.current_ind is None:
            self.current_ind = 0
        else:
            self.current_ind = (self.current_ind - 1) % self.size()
        self.select_line(self.current_ind)

    def select_line(self, line_ind: int):
        if (line_ind < 0 or
            line_ind >= self.size()):
            return
        self.select_clear(0, tk.END)
        self.select_set(line_ind)
        self.activate(line_ind)

    def set_current_line_highlight(self, enable: bool):
        if not self.current_ind is None:
            self.set_line_highlight(self.current_ind, enable)

    def set_line_highlight(self, line_ind: int, enable: bool): 
        if (line_ind < 0 or
            line_ind >= self.size()):
            return
        if enable:
            self.itemconfig(line_ind, bg='lavender')
        else:
            self.itemconfig(line_ind, bg='white')

    def clear_highlight(self):
        for line_ind in range(0,self.size()):
            self.itemconfig(line_ind, bg='white')


class OptionSelector(tk.Frame):
    def __init__(self, master=None, cnf={}, **kwargs):
        super().__init__(master, cnf, **kwargs)

        self._label = tk.Label(self)
        self._label.pack(side=tk.TOP, fill=tk.X)

        self._scroll_listbox = ScrollListbox(self, **kwargs)
        self._scroll_listbox.pack(side=tk.BOTTOM, fill=tk.BOTH, expand=True)
        self._selected_line_ind = self._scroll_listbox.current_ind

        self._scroll_listbox.bind('<Return>', self.select_current_line, add='+')
        self._scroll_listbox.bind('<Double-Button-1>', self.select_current_line, add='+')
        self._scroll_listbox.bind('<FocusOut>', self.__on_focus_out, add='+')

    def set_name(self, name: str):
        self._label.config(text=name)

    def insert(self, index, *elements):
        self._scroll_listbox.insert(index, *elements)

    def delete(self, first, last = None):
        self._scroll_listbox.delete(first, last)

    def get_current(self) -> Any:
        current_ind = self._scroll_listbox.curselection()
        return self._scroll_listbox.get(current_ind)

    def get_selected(self) -> Any:
        if not self._selected_line_ind is None:
            return self._scroll_listbox.get(self._selected_line_ind)

    def get_current_ind(self) -> int | None:
        return self._scroll_listbox.current_ind

    def get_selected_ind(self) -> int | None:
        return self._selected_line_ind

    def select_line(self, line_ind: int | None):
        if self._selected_line_ind is not None:
            self._scroll_listbox.set_line_highlight(self._selected_line_ind, False)
        if isinstance(line_ind, int):
            if line_ind < 0 or line_ind > self._scroll_listbox.size():
                logger.error('Line ind to select is out of range')
                return
            self._scroll_listbox.set_line_highlight(line_ind, True)
        self._selected_line_ind = line_ind

    def select_current_line(self, event=None):
        if not self._selected_line_ind is None:
            self._scroll_listbox.set_line_highlight(self._selected_line_ind, False)
        self._scroll_listbox.set_current_line_highlight(True)
        self._selected_line_ind = self._scroll_listbox.current_ind

    def update(self):
        self._scroll_listbox.clear_highlight()
        if not self._selected_line_ind is None:
            self._scroll_listbox.set_line_highlight(self._selected_line_ind, True)

    def __on_focus_out(self, event=None):
        self._scroll_listbox.current_ind = self._selected_line_ind 


class InfoDisplay(tk.Frame):
    def __init__(self, master=None, **kwargs):
        super().__init__(master)
        kwargs.setdefault('state', tk.DISABLED)

        self.label = tk.Label(self)
        self.label.pack(side=tk.TOP, fill=tk.X)

        self.text = tk.Text(self, **kwargs)
        self.text.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)

        self.scrollbar = tk.Scrollbar(self, orient=tk.VERTICAL, command=self.text.yview)
        self.scrollbar.pack(side=tk.RIGHT, fill=tk.Y)

        self.text.config(yscrollcommand=self.scrollbar.set)

    def set_name(self, name: str):
        self.label.config(text=name)

    def insert(self, index, *elements):
        self.text.config(state=tk.NORMAL)
        self.text.insert(index, *elements)
        self.text.config(state=tk.DISABLED)

    def clear(self):
        self.text.config(state=tk.NORMAL)
        self.text.delete(1.0, tk.END)
        self.text.config(state=tk.DISABLED)


#Button for changing stage
class NextButton(tk.Button):
    #epoint - event point
    #Button generates event <<Complete>> to point Stage to change Stage
    #But Stage listens StageGUI, not concrete button. Stage couldn't hear <<Complete>> event
    #if it is generated only inside button, due tkinter events logic. 
    #So Event needs to be gereated inside StageGUI. And there are two options:
    #1 StageGUI listens buttons <<Complete>> and generates <<Complete>> itself
    #2 Button generate <<Complete>> directly inside Stage (epoint is needed)
    #I choose 2 because I liked it more.
    def __init__(self, master=None, epoint=None, **kwargs):
        self._epoint = self if epoint is None else epoint
        super().__init__(master, text='next', command=lambda: self._epoint.event_generate('<<Complete>>'), **kwargs)


#The same as NextButton, but with additional 'direction': back (<<GoBack>> event)
#It might seem weird to get event <<Complete>> for the NextButton,
#but <<GoBack>> for the PreviousButton. But fate decided so...
class BackNextButtons(tk.Frame):
    def __init__(self, master=None, epoint=None, **kwargs):
        super().__init__(master, **kwargs)
        self._epoint = self if epoint is None else epoint

        self.back_button = tk.Button(self, text='back', command=lambda: self._epoint.event_generate('<<GoBack>>'))
        self.next_button = tk.Button(self, text='next', command=lambda: self._epoint.event_generate('<<Complete>>'))
        self.back_button.pack(side=tk.LEFT, fill=tk.X, expand=True)
        self.next_button.pack(side=tk.RIGHT, fill=tk.X, expand=True)
