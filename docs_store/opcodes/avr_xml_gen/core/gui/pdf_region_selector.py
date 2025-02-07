import tkinter as tk
from tkinter import filedialog
from PIL import ImageTk
import pdfplumber
from typing import Iterable


# The name speaks for itself.
# Selected items stores in _selected_elements
# Pdf page should be centered, but I forgot to make it
class PDFRegionSelector(tk.Frame):
    # There is added my own key: `pdf_selection`
    # It determines if pdf choosing button will be created
    def __init__(self, master=None, cnf={}, pdf_selection=True, **kwargs):
        super().__init__(master, cnf, **kwargs)

        self.__setup_ui(pdf_selection)
        self.__setup_bindings(pdf_selection)

        self._pdf = None
        self._page = None
        elements = {'word', 'line', 'curve'}
        self._page_elements = {e: [] for e in elements}
        self._selected_elements = {e: [] for e in elements}

        self._current_page_ind = 0
        # pdf page height / image height
        self._scale_factor = 1
        self._highlight_color = 'blue'


    def __setup_ui(self, pdf_selection=True):
        self._canvas = tk.Canvas(self)
        self._canvas.pack(side=tk.TOP, fill=tk.BOTH, expand=True)

        self._button_frame = tk.Frame(self)
        self._button_frame.pack(side=tk.BOTTOM, fill=tk.X)
        button_pack = {'side': tk.LEFT,
                       'fill': tk.X,
                       'expand': True,
                       }
        if pdf_selection:
            self._load_button = tk.Button(self._button_frame, text='Load PDF')
            self._load_button.pack(button_pack)
        self._prev_page_button = tk.Button(self._button_frame, text='Previous Page')
        self._prev_page_button.pack(button_pack)
        self._next_page_button = tk.Button(self._button_frame, text='Next Page')
        self._next_page_button.pack(button_pack)


    def __setup_bindings(self, pdf_selection=True):
        self._canvas.bind("<ButtonPress-1>", self.__start_rectangle)
        self._canvas.bind("<B1-Motion>", self.__draw_rectangle)
        self._canvas.bind("<ButtonRelease-1>", self.__finish_rectangle)
        self._canvas.bind('<Configure>', self.show_page)

        if pdf_selection:
            self._load_button.config(command=self.load_pdf)
        self._prev_page_button.config(command=self.prev_page)
        self._next_page_button.config(command=self.next_page)


    def __start_rectangle(self, event):
        self._canvas.delete('selection')
        self._rect_start = (event.x, event.y)
        self._rect_ind = self._canvas.create_rectangle(event.x, event.y, event.x, event.y, fill=self._highlight_color, stipple='gray12')


    def __draw_rectangle(self, event):
        if not self._rect_ind:
            return

        self._canvas.coords(self._rect_ind, self._rect_start[0], self._rect_start[1], event.x, event.y)
        rect = self._canvas.coords(self._rect_ind)
        self._canvas.delete('selection')
        self.__select_internal_elements(rect)


    def __select_internal_elements(self, image_rect: Iterable):
        _page_rect = tuple(t * self._scale_factor for t in image_rect)
        for e_name in self._page_elements.keys():
            elements = self.__get_internal_elements(_page_rect, e_name)
            self._selected_elements[e_name] = elements

        for elements in self._selected_elements.values():
            self.__highlight_elements(elements, self._highlight_color)


    def __get_internal_elements(self, _page_rect: Iterable, element_name: str) -> list:
        if not self._page:
            return []
        x0, y0, x1, y1 = _page_rect
        internal_elements = [
            e for e in self._page_elements[element_name]
            if not (x0 > e['x1'] or
                    x1 < e['x0'] or
                    y0 > e['bottom'] or
                    y1 < e['top'])
        ]
        return internal_elements


    def __highlight_elements(self, elements: Iterable, color: str):
        for e in elements:
            x0 = e['x0'] / self._scale_factor
            y0 = e['top'] / self._scale_factor
            x1 = e['x1'] / self._scale_factor
            y1 = e['bottom'] / self._scale_factor
            self._canvas.create_rectangle(x0, y0, x1, y1, fill=color, tags='selection', stipple='gray25')


    def __finish_rectangle(self, event):
        self._canvas.delete(self._rect_ind)
        self.event_generate('<<Select>>')


    def show_page(self, event=None):
        if self._pdf and 0 <= self._current_page_ind < len(self._pdf.pages):
            self._page = self._pdf.pages[self._current_page_ind]
            self._page_elements['word'] = self._page.extract_words()
            objects = self._page.objects
            self._page_elements['curve'] = objects['curve'] if 'curve' in objects.keys() else []
            self._page_elements['line'] = objects['line'] if 'line' in objects.keys() else []
            self.__display_page_image()


    def __display_page_image(self):
        if not self._page:
            return

        self._scale_factor = self._page.height / self._canvas.winfo_height()

        page_image = self._page.to_image(resolution=300).original
        new_height = self._canvas.winfo_height()
        aspect_ratio = self._page.width / self._page.height
        new_width = int(new_height * aspect_ratio)
        page_image = page_image.resize((new_width, new_height))

        self._image = ImageTk.PhotoImage(image=page_image)
        self._canvas.delete('all')
        self._canvas.create_image(0, 0, anchor=tk.NW, image=self._image)


    def set_pdf(self, pdf_path: str):
        self._pdf = pdfplumber.open(pdf_path)
        self._current_page_ind = 0
        self.show_page()


    def load_pdf(self):
        pdf_path = filedialog.askopenfilename(filetypes=[("PDF Files", "*.pdf")])
        if pdf_path:
            self.set_pdf(pdf_path)


    def next_page(self):
        if self._pdf is None:
            return
        self._current_page_ind += 1
        self._current_page_ind %= len(self._pdf.pages)
        self.show_page()


    def prev_page(self):
        if self._pdf is None:
            return
        self._current_page_ind -= 1
        self._current_page_ind %= len(self._pdf.pages)
        self.show_page()


    def set_highlight_color(self, color: str):
        self._highlight_color = color


    def get_selected_elements(self):
        return self._selected_elements
