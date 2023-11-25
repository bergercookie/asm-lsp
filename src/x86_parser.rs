use crate::types::*;

use anyhow::anyhow;
use log::{debug, error, info, warn};
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

/// Parse the provided XML contents and return a vector of all the instructions based on that.
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
                                    let name = String::from(str::from_utf8_unchecked(&value));
                                    curr_instruction.alt_names.push(name.to_uppercase());
                                    curr_instruction.name = name;
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

    let x86_online_docs = get_x86_docs_url();
    let body = get_docs_body(&x86_online_docs).unwrap_or_default();
    let body_it = body.split("<td>").skip(1).step_by(2);

    // Parse this x86 page, grab the contents of the table + the URLs they are referring to
    // Regex to match:
    // <a href="./VSCATTERPF1DPS:VSCATTERPF1QPS:VSCATTERPF1DPD:VSCATTERPF1QPD.html">VSCATTERPF1QPS</a></td>
    //
    // let re = Regex::new(r"<a href=\"./(.*)">(.*)</a></td>")?;
    // let re = Regex::new(r#"<a href="\./(.*?\.html)">(.*?)</a>.*</td>"#)?;
    // let re = Regex::new(r"<a href='\/(.*?)'>(.*?)<\/a>.*<\/td>")?;
    let re = Regex::new(r"<a href='\/x86\/(.*?)'>(.*?)<\/a>.*<\/td>")?;
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

#[cfg(test)]
mod tests {
    use crate::x86_parser::{get_cache_dir, populate_instructions};
    #[test]
    fn test_populate_instructions() {
        let mut server = mockito::Server::new_with_port(8080);

        let mock = server
            .mock("GET", "/x86/")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body("")
            .expect(2) // 1 request for x86, 1 for x86_64
            .create();

        // Need to clear the cache file (if there is one)
        // to ensure a request is made for each test call
        let mut x86_cache_path = get_cache_dir().unwrap();
        x86_cache_path.push("x86_instr_docs.html");
        if x86_cache_path.is_file() {
            std::fs::remove_file(&x86_cache_path).unwrap();
        }
        let xml_conts_x86 = include_str!("../opcodes/x86.xml");
        assert!(populate_instructions(xml_conts_x86).is_ok());

        if x86_cache_path.is_file() {
            std::fs::remove_file(&x86_cache_path).unwrap();
        }
        let xml_conts_x86_64 = include_str!("../opcodes/x86_64.xml");
        assert!(populate_instructions(xml_conts_x86_64).is_ok());

        // Clean things up so we don't have an empty cache file
        if x86_cache_path.is_file() {
            std::fs::remove_file(&x86_cache_path).unwrap();
        }

        mock.assert();
    }
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

/// Parse the provided XML contents and return a vector of all the registers based on that.
/// If parsing fails, the appropriate error will be returned instead.
///
/// Current function assumes that the XML file is already read and that it's been given a reference
/// to its contents (`&str`).
pub fn populate_registers(xml_contents: &str) -> anyhow::Result<Vec<Register>> {
    let mut registers_map = HashMap::<String, Register>::new();

    // iterate through the XML --------------------------------------------------------------------
    let mut reader = Reader::from_str(xml_contents);
    reader.trim_text(true);

    // ref to the register that's currently under construction
    let mut curr_register = Register::default();
    let mut curr_bit_flag = RegisterBitInfo::default();

    debug!("Parsing XML contents...");
    loop {
        match reader.read_event() {
            // start event ------------------------------------------------------------------------
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    QName(b"Register") => {
                        // start of a new register
                        curr_register = Register::default();

                        // iterate over the attributes
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            match str::from_utf8(key.into_inner()).unwrap() {
                                "name" => unsafe {
                                    let name_ = String::from(str::from_utf8_unchecked(&value));
                                    curr_register.alt_names.push(name_.to_uppercase());
                                    curr_register.name = name_;
                                },
                                "altname" => unsafe {
                                    curr_register
                                        .alt_names
                                        .push(String::from(str::from_utf8_unchecked(&value)));
                                },
                                "description" => unsafe {
                                    curr_register.description =
                                        Some(String::from(str::from_utf8_unchecked(&value)));
                                },
                                "type" => unsafe {
                                    curr_register.reg_type = match RegisterType::from_str(
                                        str::from_utf8_unchecked(&value),
                                    ) {
                                        Ok(reg) => Some(reg),
                                        _ => None,
                                    }
                                },
                                "width" => unsafe {
                                    curr_register.width = match RegisterWidth::from_str(
                                        str::from_utf8_unchecked(&value),
                                    ) {
                                        Ok(width) => Some(width),
                                        _ => None,
                                    }
                                },
                                _ => {}
                            }
                        }
                    }
                    QName(b"Flags") => {} // it's just a wrapper...
                    // Actual flag bit info
                    QName(b"Flag") => {
                        curr_bit_flag = RegisterBitInfo::default();

                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            match str::from_utf8(key.into_inner()).unwrap() {
                                "bit" => unsafe {
                                    curr_bit_flag.bit =
                                        str::from_utf8_unchecked(&value).parse::<u32>().unwrap();
                                },
                                "label" => unsafe {
                                    curr_bit_flag.label =
                                        String::from(str::from_utf8_unchecked(&value));
                                },
                                "description" => unsafe {
                                    curr_bit_flag.description =
                                        String::from(str::from_utf8_unchecked(&value));
                                },
                                "pae" => unsafe {
                                    curr_bit_flag.pae =
                                        String::from(str::from_utf8_unchecked(&value));
                                },
                                "longmode" => unsafe {
                                    curr_bit_flag.long_mode =
                                        String::from(str::from_utf8_unchecked(&value));
                                },
                                _ => {}
                            }
                        }
                    }
                    _ => (), // unknown event
                }
            }
            // end event --------------------------------------------------------------------------
            Ok(Event::End(ref e)) => {
                match e.name() {
                    QName(b"Register") => {
                        // finish instruction
                        registers_map.insert(curr_register.name.clone(), curr_register.clone());
                    }
                    QName(b"Flag") => {
                        curr_register.push_flag(curr_bit_flag.clone());
                    }
                    _ => (), // unknown event
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // rest of events that we don't consider
        }
    }

    // TODO: Add to URL fields here?
    // https://wiki.osdev.org/CPU_Registers_x86 and https://wiki.osdev.org/CPU_Registers_x86-64
    // are less straightforward compared to the instruction set site

    Ok(registers_map.into_values().collect())
}

pub fn populate_name_to_register_map<'register>(
    arch: Arch,
    registers: &'register Vec<Register>,
    names_to_registers: &mut NameToRegisterMap<'register>,
) {
    for register in registers {
        for name in &register.get_associated_names() {
            names_to_registers.insert((arch, name), register);
        }
    }
}

fn get_docs_body(x86_online_docs: &str) -> Option<String> {
    // provide a URL example page -----------------------------------------------------------------
    // 1. If the cache refresh option is enabled or the cache doesn't exist, attempt to fetch the
    //    data, write it to the cache, and then use it
    // 2. Otherwise, attempt to read the data from the cache
    // 3. If invalid data is read in, attempt to remove the cache file
    let cache_refresh = args().any(|arg| arg.contains("--cache-refresh"));
    let mut x86_cache_path = match get_cache_dir() {
        Ok(cache_path) => Some(cache_path),
        Err(e) => {
            warn!("Failed to resolve the cache file path - Error: {e}.");
            None
        }
    };

    // Attempt to append the cache file name to path and see if it is valid/ exists
    let cache_exists: bool;
    if let Some(mut path) = x86_cache_path {
        path.push("x86_instr_docs.html");
        cache_exists = matches!(path.try_exists(), Ok(true));
        x86_cache_path = Some(path);
    } else {
        cache_exists = false;
    }

    let body = if cache_refresh || !cache_exists {
        match get_x86_docs_web(x86_online_docs) {
            Ok(docs) => {
                if let Some(ref path) = x86_cache_path {
                    set_x86_docs_cache(&docs, path);
                }
                docs
            }
            Err(e) => {
                error!(
                    "Failed to fetch documentation from {} - Error: {e}.",
                    x86_online_docs
                );
                return None;
            }
        }
    } else if let Some(ref path) = x86_cache_path {
        match get_x86_docs_cache(path) {
            Ok(docs) => docs,
            Err(e) => {
                error!(
                    "Failed to fetch documentation from the cache: {} - Error: {e}.",
                    path.display()
                );
                return None;
            }
        }
    } else {
        error!("Failed to fetch documentation from the cache - Invalid path.");
        return None;
    };

    // try to create the iterator to check if the data is valid
    // if the body produces an empty iterator, we attempt to clear the cache
    if body.split("<td>").skip(1).step_by(2).next().is_none() {
        error!("Invalid docs contents.");
        if let Some(ref path) = x86_cache_path {
            error!("Attempting to remove the cache file {}...", path.display());
            match std::fs::remove_file(path) {
                Ok(()) => {
                    error!("Cache file removed.");
                }
                Err(e) => {
                    error!("Failed to remove the cache file - Error: {e}.",);
                }
            }
        } else {
            error!("Unable to clear the cache, invalid path.");
        }
        return None;
    }

    Some(body)
}

fn get_cache_dir() -> anyhow::Result<PathBuf> {
    // first check if the appropriate environment variable is set
    if let Ok(path) = std::env::var("ASM_LSP_CACHE_DIR") {
        let path = PathBuf::from(path);
        // ensure the path is valid
        if path.is_dir() {
            return Ok(path);
        }
    }

    // If the environment variable isn't set or gives an invalid path, grab the home directory and build off of that
    let mut x86_cache_path = home::home_dir().ok_or(anyhow!("Home directory not found"))?;

    x86_cache_path.push(".cache");
    x86_cache_path.push("asm-lsp");

    // create the ~/.cache/asm-lsp directory if it's not already there
    fs::create_dir_all(&x86_cache_path)?;

    Ok(x86_cache_path)
}

#[cfg(not(test))]
fn get_x86_docs_url() -> String {
    String::from("https://www.felixcloutier.com/x86/")
}

#[cfg(test)]
fn get_x86_docs_url() -> String {
    String::from("http://127.0.0.1:8080/x86/")
}

fn get_x86_docs_web(x86_online_docs: &str) -> anyhow::Result<String> {
    info!(
        "Fetching further documentation from the web -> {}...",
        x86_online_docs
    );
    // grab the info from the web
    let contents = reqwest::blocking::get(x86_online_docs)?.text()?;
    Ok(contents)
}

fn get_x86_docs_cache(x86_cache_path: &PathBuf) -> Result<String, std::io::Error> {
    info!(
        "Fetching html page containing further documentation, from the cache -> {}...",
        x86_cache_path.display()
    );
    fs::read_to_string(x86_cache_path)
}

fn set_x86_docs_cache(contents: &str, x86_cache_path: &PathBuf) {
    info!("Writing to the cache file {}...", x86_cache_path.display());
    match fs::File::create(x86_cache_path) {
        Ok(mut cache_file) => {
            info!("Created the cache file {} .", x86_cache_path.display());
            match cache_file.write_all(contents.as_bytes()) {
                Ok(()) => {
                    info!("Populated the cache.");
                }
                Err(e) => {
                    error!(
                        "Failed to write to the cache file {} - Error: {e}.",
                        x86_cache_path.display()
                    );
                }
            }
        }
        Err(e) => {
            error!(
                "Failed to create the cache file {} - Error: {e}.",
                x86_cache_path.display()
            );
        }
    }
}
