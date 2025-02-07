This is a program for generating avr instructions xml

Program uses the avr datasheet, which you can find under [AVR-Instruction-Set-Manual-DS40002198A.pdf](https://ww1.microchip.com/downloads/en/DeviceDoc/AVR-InstructionSet-Manual-DS40002198.pdf)

The whole programm is split into 4 stages:

1. Data markers
2. Data extraction
3. Data analysis
4. Xml generation

For gui used [tkinter](https://docs.python.org/3/library/tkinter.html)


- - - - - - - - - - - - - - - - - -
# 1 Data managinig and structuring
- - - - - - - - - - - - - - - - - -
Stored in `core/data_processing`.
There are series of unfortunate titles such as `data_processing` or `data_processing.data_management`,
because they don't point to the exact purpose of classes. This can be changed in
the future, but so far, fate has decided.

----------------------------------------
## 1.1 Storing instruction and its forms

Each instruction form has several aspects. They are:
1. Mnemonic ('name' in xml)
2. Version (version of avr architecture)
3. Description ('summary' in xml)
4. Operands
5. SREG (status registers)
6. Opcode

I extracted aspects 1, 3, and 4 from tables and 1, 2, 5, and 6 from chapters (datasheet,
chapter 5). This is merged, so now 3 and 4 is stored as `table data`', while 5 and
6  are stored as `chapter data`. Sometimes you can see subchapter instead of chapter
because at the very beginning I wanted to name it 'subchapter'. In some cases, division
into the chapter and subchapter makes sence (in the data extraction stage). Even
though 2 (version) is extracted from a chapter, the version is chapter-data independent,
and it was just convenient to extract versions from chapter.

There is the class `data_management.InstructionForm` for storing `mnemonic`, `version`,
`table_data` and `chapter_data`. It inherits `data_management.StrictDictionary`
which is the same as default dictionary, but with fixed keys. This is helpful,
as python points tells if there are wrong or missing keys.

Instruction is stored as a dictionary with `str -> list[InstructionForm]`, where
`str` is the instruction name.

--------------
## 1.2 Context

`data_management.Context` stores all data that could be used by stages. It stores
instruction data markers, instruction data extracted from datasheet, parsed instruction
data (instruction forms), and even the datasheet file name.

Context has dictionary and works in the key-value concept, but with one feature:
keys are data type. So it could store only one copy of particular data. This was
better than a dictionary with str names or something else. During development, it
became clear that this needed to be changed for Aliases.

Some unique data (e.g `ExtractedInstructionData`, `ProcessedInstructionData`) could
just be an `Alias`, but they would have been the same key of type `Alias` if I have
left as it was. A future improvement could be to make context distinguish different
aliases as keys.

---------------------
## 1.3 Ambiguous data

In `ambiguous_data.py` you can see several default data types analogs (`dict`,
`list`, `forzenset`) but with the 'Ambiguous' prefix.

During the data extraction it became clear that it is useful to determine if a list
or set has more than one object. It was too often so I decided to create the same
classes but with a `is_ambiguous()` method and special output: it prints 'Ambiguous'
if data is ambiguous. The dictionary is ambiguous if it has ambiguous value or key.
The whole extracted instruction data is `AmbiguousDict[AmbiguousFrozen, AmbiguousList]`.

---------------------------
## 1.4 Other random classes

Observer and Subject:
I used the observer pattern for stages, so I just shoved these classes here :)

DataManager:
Is used by stage (2). It has `request()` method. The implementation depends on a
particular class. The biggest example of this is in `core/stages/data_analysis/instruction_forms_manager.py`.


- - - - - -
# 2 Stages
- - - - - -
All related to stages is stored in `core/stages`.

-----------------------------------
## 2.1 The main tasks of the stages

1. The data markers stage customizes the instruciton data markers that determine
what is useful data in the datasheet. It isn't used at this time because data extraction
stage isn't completed. Details are written in '2 Data extraction'. If you want to
see what is created for data markers, replace `app.after(100, stage2.execute)` with
`app.after(100, stage1.execute)`. This is a demonstration of a field selector. It
can catch pdf objects like images, tables and words and access to all of their attributes.

2. The data extraction stage uses data markers created in first stage to extract
instruction data from the datasheet. This stage doesn't have a modular structure
and doesn't use markers, so it should be remade.

3. Data analysis converts extracted data for direct xml compilation. This data is
a dictionary with instruction names as keys and lists of instruction forms as values.
Datasheet data isn't always ambiguous, which can be resolved by two ways: manually
by selecting needed options or programmatically by adding code to handle specific
cases. The first option is preferred.

4. Xml generation speaks for itself. It uses processed data from the data analysis
stage.

---------------------------
## 2.2 Stage concept itself
Basic stage classes stored in `stage.py`

Summary:

The whole programm is split into stages. Each stage has next stage (it could be None),
`execute()`, and `try_complete()` methods. It require `Context` (1.2) for initialization.

At the very beginning we `execute()` the first stage. By default, the stage tries
to be completed by `try_complete()` after the `<<Complete>>` event. If it is completed,
the stage executes the next stage. Otherwise, its work is continued.

`Stage` also has a `permanently_completed` variant. This is needed for `BidirectionStage`,
which could go forwards and back. If `Stage` is permanently completed, it can't be
`execute()`d.

More details:
In addition to the `Stage` class, there is also `StageTask`, `StageGUI` and `BidirectionalStage`.
`StageTask` is used by `try_complete()` to check if completion conditions are satisfied
by method `is_completed()`. It also requires `Context`.

`StageGUI` is used if we need gui :). It has `enable()` and `disable()` methods.
It requires `DataManager`. (1.4) `Stage.execute()` calls `StageGUIStage` GUI generates
a `<<Complete>>` event. After generating this event, `Stage` executes `try_complete()`.

In fact, `StageTask` and `StageGUI` are optional. See `xml_generation.py`.

`BidirectionalStage` is the same as default stage, except that it also has the previous_stage.
It inherits `Stage`.

----------------------------
## 2.3 Stages implementation

1. Data markers has one stage `SettingHeader`. It determines the height of the pdf
header. This is just a demonstration. For gui,`PDFRegionSelector`is used.

2. Data extraction has one big process of data extraction, no more. As I mentioned
in 2.1, it could be cleaned up. It has gui with a 'waiting' caption.

3. Data analysis has `InstructionBuilding` class. It is `BidirectionalStage`. An
`InstructionBuilding` instance is created for each instruction. It automatically
generates one singular instruction form and completes itself, if instruction data
isn't ambiguous (not `is_ambiguous()`, see `AmbiguousData` in 1.3), otherwise its
GUI is opened. In the GUI, the user creates forms and selects correct form aspects.
A future imrprovement could be adding saving for selected options. In `app.py` you
can see `InstructionBuildingInitializing`. It is needed for creating `InstructionBuilding`
instances.

4. Xml generation has `XMLGeneration` which uses `InstructionXMLBuilder`. It is a
realy simple stage even without GUI and Task (task should be added to check if all
is good)
