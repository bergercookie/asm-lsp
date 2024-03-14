import pandas as pd
import numpy as np
import re
import xml.etree.cElementTree as ET
from xml.dom import minidom

def splitargs(x):
    tokens = x.split(" ")
    if len(tokens) == 2:
        return tokens[1].split(",")
    if len(tokens) > 2:
        raise NotImplemented()

def clean_str(x):
    x = str(x)
    x = x.replace("²", "")
    x = x.replace("³", "")
    return x

def parse_csv(filename:str) -> pd.DataFrame:
    df = pd.read_csv(filename)
    df["instruction"] = df["instructionform"].apply(lambda x: x.split(" ")[0])
    df["arguments"] = df["instructionform"].apply(lambda x: splitargs(x))
    return df

def parse_descr_csv(filename:str) -> pd.DataFrame:
    df = pd.read_csv(filename)
    return df

def make_xml(df: pd.DataFrame, descr_df: pd.DataFrame, dst_filename: str):
    root = ET.Element("InstructionSet", name="z80")
    instruction_list = df["instruction"].unique().tolist()
    arg_pattern = r'(?P<arg1>[^"]*),(?P<arg2>[^"]*)'
    for instruction_name in instruction_list:
        description = descr_df[descr_df["instruction"] == instruction_name]["description"]
        if not description.empty:
            description = description.values[0].replace('"', '')
        else:
            description = ""
        instruction_elem = ET.Element("Instruction", name=instruction_name.lower(), summary=description)
        dff = df[df["instruction"] == instruction_name]
        for _, row in dff.iterrows():
            # Add spaces between comma separated args so we can
            # create a valid URL fragment later on
            instr_form = clean_str(row["instructionform"])
            if ',' in instr_form:
                instr_form = re.sub(arg_pattern, r'\1, \2', instr_form)
            instructionform_elem = ET.Element("InstructionForm", form=instr_form, z80name=instruction_name.lower())
            # opcodes
            opcodes = row["opcode"].split(" ")
            encoding_elem = ET.Element("Encoding")
            for opcode in opcodes:
                opcode_elem = ET.Element("Opcode", byte=opcode)
                encoding_elem.append(opcode_elem)
            instructionform_elem.append(encoding_elem)
            # arguments and/or registers
            if row["arguments"] is not None and len(row["arguments"]) > 0:
                arg_elem = None
                args = row["arguments"][0].split(",")
                for arg in args:
                    if arg is None or arg is np.nan:
                        continue
                    arg_ = arg.replace("(", "")
                    arg_ = arg.replace(")", "")
                    if arg_ in "ABCDEHLIXIYSP":
                        arg_elem = ET.Element("Argument", reg=arg.lower())
                    else:
                        arg_elem = ET.Element("Argument", arg=arg.lower())
                if arg_elem is not None:
                    instructionform_elem.append(arg_elem)
            # Prefer missing elements to nan to avoid parsing errors in the main program
            if clean_str(row["timing_z80"]) != "nan":
                instructionform_elem.append(ET.Element("TimingZ80", value=clean_str(row["timing_z80"])))
            if clean_str(row["timing_z80_m1"]) != "nan":
                instructionform_elem.append(ET.Element("TimingZ80M1", value=clean_str(row["timing_z80_m1"])))
            if clean_str(row["timing_r800"]) != "nan":
                instructionform_elem.append(ET.Element("TimingR800", value=clean_str(row["timing_r800"])))
            if clean_str(row["timing_r800_wait"]) != "nan":
                instructionform_elem.append(ET.Element("TimingR800Wait", value=clean_str(row["timing_r800_wait"])))
            instruction_elem.append(instructionform_elem)
        root.append(instruction_elem)
    tree = ET.ElementTree(root)
    tree.write(dst_filename)
    # Format the file to make it human readable.
    dom = minidom.parse(dst_filename)
    pretty_xml = dom.toprettyxml()
    with open(dst_filename, "w") as fp:
        fp.write(pretty_xml)
    print(f"Output written to {dst_filename}.")


if __name__ == "__main__":
    df = parse_csv("z80.csv")
    descr_df = parse_descr_csv("z80_instr_descr.csv")
    make_xml(df, descr_df, "../opcodes/z80.xml")
