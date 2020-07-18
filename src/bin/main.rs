use look_asm::*;

use anyhow::anyhow;
use log::{error, info, warn};
use quick_xml::events::attributes::Attribute;
use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;
use reqwest;
use std::str;
use std::str::FromStr;

// main -------------------------------------------------------------------------------------------
pub fn main() -> anyhow::Result<()> {
    // initialisation -----------------------------------------------------------------------------
    env_logger::init();

    let mut buf = Vec::new();

    // initialise the instruction set
    let mut instruction_set = InstructionSet::new();

    // Embed the codes in the binary
    let opcodes_x86 = include_str!("../../opcodes/x86.xml");
    let opcodes_x86 = include_str!("../../opcodes/x86_64.xml");

    // iterate through the XML --------------------------------------------------------------------
    let mut reader = Reader::from_str(opcodes_x86);
    reader.trim_text(true);

    // ref to the instruction that's currently under construction
    let mut curr_instruction = Instruction::default();
    let mut curr_instruction_form = InstructionForm::default();

    // TODO generalise it
    let curr_xml = &opcodes_x86;
    loop {
        match reader.read_event(&mut buf) {
            // start event ------------------------------------------------------------------------
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    b"Instruction" => {
                        // start of a new instruction
                        curr_instruction = Instruction::default();

                        // iterate over the attributes
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            match str::from_utf8(key).unwrap() {
                                "name" => unsafe {
                                    curr_instruction.name =
                                        String::from(str::from_utf8_unchecked(&value));
                                },
                                "summary" => unsafe {
                                    curr_instruction.summary =
                                        String::from(str::from_utf8_unchecked(&value));
                                },
                                _ => {}
                            }
                        }
                    }
                    b"InstructionForm" => {
                        // Read the attributes
                        //
                        // <xs:attribute name="gas-name" type="xs:string" use="required" />
                        // <xs:attribute name="go-name" type="xs:string" />
                        // <xs:attribute name="mmx-mode" type="MMXMode" />
                        // <xs:attribute name="xmm-mode" type="XMMMode" />
                        // <xs:attribute name="cancelling-inputs" type="xs:boolean" />
                        // <xs:attribute name="nacl-version" type="NaClVersion" />
                        // <xs:attribute name="nacl-zero-extends-outputs" type="xs:boolean" />

                        // new instruction
                        curr_instruction_form = InstructionForm::default();

                        // iterate over the attributes
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            match str::from_utf8(key).unwrap() {
                                "gas-name" => unsafe {
                                    curr_instruction_form.gas_name =
                                        Some(String::from(str::from_utf8_unchecked(&value)));
                                },
                                "go-name" => unsafe {
                                    curr_instruction_form.go_name =
                                        Some(String::from(str::from_utf8_unchecked(&value)));
                                },
                                "mmx-mode" => unsafe {
                                    let value_ = value.as_ref();
                                    curr_instruction_form.mmx_mode = Some(
                                        MMXMode::from_str(str::from_utf8_unchecked(&value_))?
                                    );
                                },
                                "xmm-mode" => unsafe {
                                    let value_ = value.as_ref();
                                    curr_instruction_form.xmm_mode = Some(
                                        XMMMode::from_str(str::from_utf8_unchecked(&value_))?
                                    );
                                },
                                "cancelling-inputs" => match str::from_utf8(&value).unwrap() {
                                    "true" => curr_instruction_form.cancelling_inputs = Some(true),
                                    "false" => {
                                        curr_instruction_form.cancelling_inputs = Some(false)
                                    }
                                    _ => {
                                        return Err(anyhow!(
                                            "Unknown value for XML attribute {}",
                                            "nacl-zero-extends-outputs"
                                        ))
                                    }
                                },
                                "nacl-version" => {
                                    curr_instruction_form.nacl_version =
                                        value.as_ref().first().cloned();
                                }
                                "nacl-zero-extends-outputs" => {
                                    match str::from_utf8(&value).unwrap() {
                                        "true" => {
                                            curr_instruction_form.nacl_zero_extends_outputs =
                                                Some(true)
                                        }
                                        "false" => {
                                            curr_instruction_form.nacl_zero_extends_outputs =
                                                Some(false)
                                        }
                                        _ => {
                                            return Err(anyhow!(
                                                "Unknown value for XML attribute {}",
                                                "nacl-zero-extends-outputs"
                                            ))
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    b"Encoding" => {} // TODO
                    _ => (),          // unknown event
                }
            }
            Ok(Event::Empty(ref e)) => {
                match e.name() {
                    b"ISA" => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            match str::from_utf8(key).unwrap() {
                                "id" => unsafe {
                                    curr_instruction_form.isa = Some(
                                        ISA::from_str(str::from_utf8_unchecked(&value.as_ref()))
                                            .expect(&format!(
                                                "Unexpected ISA variant - {}",
                                                str::from_utf8_unchecked(&value)
                                            )),
                                    )
                                },
                                _ => (),
                            }
                        }
                    }
                    b"Operand" => {
                        let mut type_: OperandType;
                        let mut extended_size = None;
                        let mut input = None;
                        let mut output = None;

                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            match str::from_utf8(key).unwrap() {
                                "type" => {
                                    type_ = OperandType::from_str(str::from_utf8(&value)?)?;
                                }
                                "input" => match str::from_utf8(&value).unwrap() {
                                    "true" => input = Some(true),
                                    "false" => input = Some(false),
                                    _ => return Err(anyhow!("Unknown value for operand type")),
                                },
                                "output" => match str::from_utf8(&value).unwrap() {
                                    "true" => output = Some(true),
                                    "false" => output = Some(false),
                                    _ => return Err(anyhow!("Unknown value for operand type")),
                                },
                                "extended-size" => {
                                    extended_size =
                                        Some(str::from_utf8(value.as_ref()).unwrap().parse::<usize>()?);
                                }
                                _ => (), // unknown event
                            }
                        }
                        // TODO - Push into instruction form

                        curr_instruction_form.operands.push(
                            Operand {
                                type_: type_.clone(),
                                extended_size,
                                input,
                                output,
                            }
                        )
                    }
                    _ => (), // unknown event
                }
            }
            // end event --------------------------------------------------------------------------
            Ok(Event::End(ref e)) => {
                match e.name() {
                    b"Instruction" => {
                        // finish instruction
                        instruction_set
                            .insert(curr_instruction.name.clone(), curr_instruction.clone());
                    }
                    b"InstructionForm" => {
                        curr_instruction.push_form(curr_instruction_form.clone());
                    }
                    _ => (), // unknown event
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // rest of events that we don't consider
        }
    }
    info!("Finished parsing XML file -> {}", curr_xml);

    // provide a URL example page -----------------------------------------------------------------
    // parse this x86 page, grab the contents of the table + the URLs they are referring to
    let x86_online_docs = String::from("https://www.felixcloutier.com/x86/");
    let body = reqwest::blocking::get(&x86_online_docs)?.text()?;

    // skip first line
    let body_it = body.split("<td>").skip(1).step_by(2);

    // Regex to match:
    // <a href="./VSCATTERPF1DPS:VSCATTERPF1QPS:VSCATTERPF1DPD:VSCATTERPF1QPD.html">VSCATTERPF1QPS</a></td>
    //
    // let re = Regex::new(r"<a href=\"./(.*)">(.*)</a></td>")?;
    let re = Regex::new(r#"<a href="\./(.*?\.html)">(.*?)</a>.*</td>"#)?;
    for line in body_it {
        // take it step by step.. match a small portion of the line first...
        let caps = re.captures(&line).unwrap();
        let url_suffix = caps.get(1).map_or("", |m| m.as_str());
        let instruction_name = caps.get(2).map_or("", |m| m.as_str());

        // add URL to the corresponding instruction
        match instruction_set.get_mut(instruction_name) {
            None => (), // key not found
            Some(url_field) => url_field.url = Some(x86_online_docs.clone() + url_suffix),
        }
    }

    println!("instruction_set: {:#?}", instruction_set);
    Ok(())
}
