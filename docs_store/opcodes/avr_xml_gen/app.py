from core.logging import setup_logging
setup_logging()

from core.data_processing import Context, InstructionDataMarkers, SourceInfo, ExtractedInstructionsData, ProcessedInstructionsData
from core.stages.data_markers import SettingHeader
from core.stages.data_extraction import DataExtraction
from core.stages.data_analysis import InstructionBuilding
from core.stages.xml_generation import XMLGeneration
from core.stages import Stage, BidirectionalStage, StageGUI, StageTask
from core.data_processing.data_management import DataManager
import tkinter as tk
import os


class InstructionBuildingInitializingTask(StageTask):
    def is_completed(self) -> bool:
        return True


class InstructionBuildingInitializingGUI(StageGUI):
    def __init__(self, master=None, cnf=..., **kwargs) -> None:
        super().__init__(None, master, cnf, **kwargs)
        self.__setup_ui()
        self.__setup_bindings()

    def __setup_ui(self):
        self._label = tk.Label(self, text='Now we need to make up the forms\n' \
                                          'Some instructions data is ambiguous\n' \
                                          'In that case you need to make instruction froms manually'
                               )
        self._button = tk.Button(self, text='Resolve ambiguoity')
        self._label.pack(side=tk.TOP, fill=tk.BOTH, expand=True)
        self._button.pack(side=tk.BOTTOM, fill=tk.X, expand=True)

    def __setup_bindings(self):
        self._button.config(command=lambda: self.event_generate('<<Complete>>'))


class InstructionBuildingInitializing(Stage):
    def __init__(self, stages: set[Stage], context: Context, master=None, cnf=..., **kwargs) -> None:
        super().__init__(context, master, cnf, **kwargs)
        self.set_gui(InstructionBuildingInitializingGUI())
        self.set_task(InstructionBuildingInitializingTask(self._context))
        self._stages = stages

    def execute(self) -> None:
        extracted_data = self._context[ExtractedInstructionsData]
        stages: list[BidirectionalStage] = []
        for instruction_name in extracted_data.keys():
            stages.append(InstructionBuilding(instruction_name, self._context))
        for i in range(1, len(stages)):
            stages[i-1].set_next(stages[i])
            stages[i].set_previous(stages[i-1])
        stages[-1].set_next(self._next_stage)
        self._next_stage = stages[0]
        self._stages.update(stages)
        if self._gui is not None:
            self._gui.enable(expand=True)


class EndStageGUI(StageGUI):
    def __init__(self, data_manager: DataManager | None = None, master=None, cnf=..., **kwargs) -> None:
        super().__init__(data_manager, master, cnf, **kwargs)
        self.label = tk.Label(self, text='Work is completed')
        self.label.pack()


class EndStage(Stage):
    def __init__(self, context: Context, master=None, cnf=..., **kwargs) -> None:
        super().__init__(context, master, cnf, **kwargs)
        self._gui = EndStageGUI()


def main():
    app = tk.Tk()

    context = Context()
    context.record(ExtractedInstructionsData())
    context.record(ProcessedInstructionsData())
    source_info = SourceInfo()
    source_info['pdf_path'] = os.path.abspath('./AVR-Instruction-Set-Manual-DS40002198A.pdf')
    context.record(source_info)
    context.record(InstructionDataMarkers())

    stages: set[Stage] = set()
    stage1 = SettingHeader(context)
    stage2 = DataExtraction(context)
    stage3 = InstructionBuildingInitializing(stages, context)
    stage4 = XMLGeneration(context)
    stage5 = EndStage(context)

    stage1.set_next(stage2)
    stage2.set_next(stage3)
    stage3.set_next(stage4)
    stage4.set_next(stage5)
    app.after(100, stage2.execute)
    app.mainloop()

main()
