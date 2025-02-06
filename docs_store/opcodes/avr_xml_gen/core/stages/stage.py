from abc import ABC, abstractmethod
import tkinter as tk
import logging
from ..data_processing import Context, DataManager


logger=logging.getLogger(__name__)


class StageGUI(ABC, tk.Frame):
    def __init__(self, data_manager: DataManager | None=None, master=None, cnf={}, **kwargs) -> None:
        super().__init__(master, cnf={}, **kwargs)
        self._kwargs: dict = {}
        self._cnf: dict = {}
        self._data_manager = data_manager

    def enable(self, cnf={}, **kwargs) -> None:
        if kwargs or cnf:
            self._kwargs = kwargs
            self._cnf = cnf
        else:
            if self._kwargs:
                kwargs = self._kwargs
            if self._cnf:
                cnf = self._cnf
        self.pack(cnf, **kwargs)

    def pack(self, cnf={}, **kwargs):
        super().pack(cnf, **kwargs)
        self._kwargs = kwargs
        self._cnf = cnf

    def disable(self) -> None:
        self.pack_forget()


class StageTask(ABC):
    def __init__(self, context: Context) -> None:
        self._context = context

    @abstractmethod
    def is_completed(self) -> bool:
        pass


class Stage(ABC):
    def __init__(self, context: Context, master=None, cnf={}, **kwargs) -> None:
        self._context = context
        self._gui: StageGUI | None = None
        self._task: StageTask | None = None
        self._next_stage: Stage | None = None
        self.permanently_completed: bool = False

    def set_task(self, task: StageTask) -> None:
        self._task = task

    def set_next(self, next_stage: 'Stage | None') -> None:
        self._next_stage = next_stage

    def set_gui(self, gui: 'StageGUI | None') -> None:
        self._gui = gui
        if self._gui is not None:
            self._gui.bind('<<Complete>>', self.try_complete, add='+')

    def get_next(self) -> 'Stage | None':
        return self._next_stage

    def execute(self) -> None:
        if self._gui is not None:
            self._gui.enable(fill=tk.BOTH, expand=True)
            self._gui.update()

    def try_complete(self, event=None) -> None:
        if self._task is None:
            logger.error(f'Task isn\'t setted')
            return
        if not self._task.is_completed():
            logger.info(f'Can\'t complete stage({self}): task({self._task}) isn\'t completed')
            return
        self._complete(event)

    def _complete(self, event=None) -> None:
        while True:
            if self._next_stage is None:
                logger.error('Can\'t execute the next stage due its absence')
                return
            elif not self._next_stage.permanently_completed:
                if self._gui is not None:
                    self._gui.disable()
                self._next_stage.execute()
                return
            else:
                self.set_next(self._next_stage.get_next())


#there is two direcitons: forward and backward
#forward is 'complete', backward is 'go_back'
#naming may seem weird, I commented it in core.gui.common_ui.BackNextButton
class BidirectionalStage(Stage):
    def __init__(self, context: Context, master=None, cnf={}, **kwargs) -> None:
        super().__init__(context, master, cnf, **kwargs)
        self._previous_stage: 'BidirectionalStage | Stage | None' = None
        self._next_stage: 'BidirectionalStage | Stage | None' = self._next_stage

    def set_gui(self, gui: 'StageGUI | None'):
        super().set_gui(gui)
        if self._gui is not None:
            self._gui.bind('<<GoBack>>', self.try_go_back)

    def set_next(self, next_stage: 'BidirectionalStage | Stage | None') -> None:
        self._next_stage = next_stage

    def set_previous(self, previous_stage: 'BidirectionalStage | Stage | None') -> None:
        self._previous_stage = previous_stage

    def get_next(self) -> 'BidirectionalStage | Stage | None':
        return self._next_stage

    def get_previous(self) -> 'BidirectionalStage | Stage | None':
        return self._previous_stage

    #In fact I could just make one function instead of two (try_go_back, _go_back)
    #But I created two functions for the sake of symmetry with try_complete, _complete
    def try_go_back(self, event=None) -> None:
        self._go_back(event)

    def _go_back(self, event=None) -> None:
        while True:
            if self._previous_stage is None:
                logger.error(f'Can\'t execute the previous stage due its absence')
                return
            elif not self._previous_stage.permanently_completed:
                if self._gui is not None:
                    self._gui.disable()
                self._previous_stage.execute()
                return
            elif not isinstance(self._previous_stage, BidirectionalStage):
                logger.error(f'Previous stage is unreachable')
                return
            else:
                self.set_previous(self._previous_stage.get_previous())
