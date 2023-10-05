use crate::types::*;

use anyhow::anyhow;
use log::{debug, error, info};
use quick_xml::events::attributes::Attribute;
use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::Reader;
use regex::Regex;
use reqwest;
use std::collections::HashMap;
use std::env::args;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::str;
use std::str::FromStr;

/// Parse the provided XML contents and return a vector of all the instrucitons based on that.
/// If parsing fails, the appropriate error will be returned instead.
///
/// Current function assumes that the XML file is already read and that it's been given a reference
/// to its contents (`&str`).
pub fn populate_instructions(xml_contents: &str) -> anyhow::Result<Vec<Instruction>> {
    // initialise the instruction set
    let mut instructions_map = HashMap::<String, Instruction>::new();

    // iterate through the XML --------------------------------------------------------------------
    let mut reader = Reader::from_str(xml_contents);
    reader.trim_text(true);

    // ref to the instruction that's currently under construction
    let mut curr_instruction = Instruction::default();
    let mut curr_instruction_form = InstructionForm::default();

    debug!("Parsing XML contents...");
    loop {
        match reader.read_event() {
            // start event ------------------------------------------------------------------------
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    QName(b"Instruction") => {
                        // start of a new instruction
                        curr_instruction = Instruction::default();

                        // iterate over the attributes
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            match str::from_utf8(key.into_inner()).unwrap() {
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
                    QName(b"InstructionForm") => {
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
                            match str::from_utf8(key.into_inner()).unwrap() {
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
                                    curr_instruction_form.mmx_mode =
                                        Some(MMXMode::from_str(str::from_utf8_unchecked(value_))?);
                                },
                                "xmm-mode" => unsafe {
                                    let value_ = value.as_ref();
                                    curr_instruction_form.xmm_mode =
                                        Some(XMMMode::from_str(str::from_utf8_unchecked(value_))?);
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
                    QName(b"Encoding") => {} // TODO
                    _ => (),                 // unknown event
                }
            }
            Ok(Event::Empty(ref e)) => {
                match e.name() {
                    QName(b"ISA") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if str::from_utf8(key.into_inner()).unwrap() == "id" {
                                unsafe {
                                    curr_instruction_form.isa = Some(
                                        ISA::from_str(str::from_utf8_unchecked(value.as_ref()))
                                            .unwrap_or_else(|_| {
                                                panic!(
                                                    "Unexpected ISA variant - {}",
                                                    str::from_utf8_unchecked(&value)
                                                )
                                            }),
                                    )
                                }
                            }
                        }
                    }
                    QName(b"Operand") => {
                        let mut type_ = OperandType::k; // dummy initialisation
                        let mut extended_size = None;
                        let mut input = None;
                        let mut output = None;

                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            match str::from_utf8(key.into_inner()).unwrap() {
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
                                    extended_size = Some(
                                        str::from_utf8(value.as_ref()).unwrap().parse::<usize>()?,
                                    );
                                }
                                _ => (), // unknown event
                            }
                        }

                        curr_instruction_form.operands.push(Operand {
                            type_,
                            extended_size,
                            input,
                            output,
                        })
                    }
                    _ => (), // unknown event
                }
            }
            // end event --------------------------------------------------------------------------
            Ok(Event::End(ref e)) => {
                match e.name() {
                    QName(b"Instruction") => {
                        // finish instruction
                        instructions_map
                            .insert(curr_instruction.name.clone(), curr_instruction.clone());
                    }
                    QName(b"InstructionForm") => {
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

    // provide a URL example page -----------------------------------------------------------------
    // 1. If the cache refresh option is enabled or the cache doesn't exist, attempt to fetch the
    //    data, write it to the cache, and then use it
    //      - parse this x86 page, grab the contents of the table + the URLs they are referring to
    // 2. Otherwise, attempt to read the data from the cache
    //
    // Do we want to change behavior on error and just not include the web info?
    //  e.g change body over to an option and only mutate the map if it's Some(_)
    let cache_refresh = args() // replace with an actual way to check CLI args
        .into_iter()
        .fold(false, |accum, arg| accum || arg.contains("refresh"));
    let x86_online_docs = String::from("https://www.felixcloutier.com/x86/");
    let mut x86_cache_path = match get_cache_dir() {
        Ok(cache_path) => cache_path,
        Err(e) => {
            error!("Failed to resolve the cache file path - Error: {e}\n");
            return Err(e);
        }
    };

    // At this point we have a valid directory, now append the file name
    x86_cache_path.push("x86_docs.cache");
    let cache_exists = match x86_cache_path.try_exists() {
        Ok(true) => true,
        _ => false,
    };

    let body = if cache_refresh || !cache_exists {
        debug!(
            "Fetching further documentation from the web -> {}...",
            x86_online_docs
        );
        // grab the info from the web
        let contents = reqwest::blocking::get(&x86_online_docs.clone())?.text()?;
        // attempt to populate the cache
        match fs::File::create(x86_cache_path.clone()) {
            Ok(mut cache_file) => {
                info!("Created the cache file...");
                match cache_file.write_all(contents.as_bytes()) {
                    Ok(()) => {
                        info!("Populated the cache\n");
                    }
                    Err(e) => {
                        error!(
                            "Failed to write to the cache file {} - Error: {e}\n",
                            x86_cache_path.display()
                        );
                    }
                }
            }
            Err(e) => {
                error!(
                    "Failed to create the cache file {} - Error: {e}\n",
                    x86_cache_path.display()
                );
            }
        }
        contents
    } else {
        // read the cache from the fs
        debug!(
            "Fetching further documentation from the cache -> {}...",
            x86_cache_path.display()
        );
        fs::read_to_string(x86_cache_path)?
    };

    // skip first line
    let body_it = body.split("<td>").skip(1).step_by(2);

    // Regex to match:
    // <a href="./VSCATTERPF1DPS:VSCATTERPF1QPS:VSCATTERPF1DPD:VSCATTERPF1QPD.html">VSCATTERPF1QPS</a></td>
    //
    // let re = Regex::new(r"<a href=\"./(.*)">(.*)</a></td>")?;
    // let re = Regex::new(r#"<a href="\./(.*?\.html)">(.*?)</a>.*</td>"#)?;
    let re = Regex::new(r"<a href='\/(.*?)'>(.*?)<\/a>.*<\/td>")?;
    for line in body_it {
        // take it step by step.. match a small portion of the line first...
        let caps = re.captures(line).unwrap();
        let url_suffix = caps.get(1).map_or("", |m| m.as_str());
        let instruction_name = caps.get(2).map_or("", |m| m.as_str());

        // add URL to the corresponding instruction
        match instructions_map.get_mut(instruction_name) {
            None => (), // key not found
            Some(instruction) => instruction.url = Some(x86_online_docs.clone() + url_suffix),
        }
    }

    Ok(instructions_map.into_values().collect())
}

pub fn populate_name_to_instruction_map<'instruction>(
    arch: Arch,
    instructions: &'instruction Vec<Instruction>,
    names_to_instructions: &mut NameToInstructionMap<'instruction>,
) {
    for instruction in instructions {
        for name in &instruction.get_associated_names() {
            names_to_instructions.insert((arch, name), instruction);
        }
    }
}

fn get_cache_dir() -> anyhow::Result<PathBuf> {
    // grab the home directory and build off of that
    let home_path = std::env::var("HOME")?;
    let mut x86_cache_path = PathBuf::from(home_path);

    x86_cache_path.push(".cache");
    x86_cache_path.push("asm-lsp");

    // create the ~/.cache/asm-lsp directory if it's not already there
    fs::create_dir_all(x86_cache_path.clone())?;

    Ok(x86_cache_path)
}
