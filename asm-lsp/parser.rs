use crate::ustr;
use std::collections::HashMap;
use std::env::args;
use std::fs;
use std::io::Write;
use std::iter::Peekable;
use std::path::PathBuf;
use std::str::{FromStr, Lines};

use crate::types::{
    Arch, Assembler, Directive, Instruction, InstructionForm, MMXMode, NameToDirectiveMap,
    NameToInstructionMap, NameToRegisterMap, Operand, OperandType, Register, RegisterBitInfo,
    RegisterType, RegisterWidth, XMMMode, Z80Timing, Z80TimingInfo, ISA,
};
use crate::InstructionAlias;

use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use quick_xml::escape::unescape;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::Reader;
use regex::Regex;
use reqwest;
use url_escape::encode_www_form_urlencoded;

/// Parse all of the register information witin the documentation file
///
/// Current function assumes that the RST file is already read and that it's been given a reference
/// to its contents (`&str`).
///
/// # Panics
///
/// This function is highly specialized to parse a specific file and will panic
/// for most mal-formed/unexpected inputs
#[must_use]
pub fn populate_riscv_registers(rst_contents: &str) -> Vec<Register> {
    enum ParseState {
        FileStart,
        SectionStart,
        TableStart,
        TableSeparator,
        TableEntry,
        TableEnd,
        FileEnd,
    }
    let mut parse_state = ParseState::FileStart;
    let mut registers = Vec::new();
    let mut curr_reg_type: Option<RegisterType> = None;
    let mut lines = rst_contents.lines().peekable();

    loop {
        match parse_state {
            ParseState::FileStart => {
                let file_header = lines.next().unwrap();
                assert!(file_header.eq("Register Definitions"));
                let separator = lines.next().unwrap();
                assert!(separator.starts_with('='));
                consume_empty_lines(&mut lines);
                parse_state = ParseState::SectionStart;
            }
            ParseState::SectionStart => {
                let section_header = lines.next().unwrap();
                if section_header.contains("Integer") {
                    curr_reg_type = Some(RegisterType::GeneralPurpose);
                } else if section_header.contains("Floating Point") {
                    curr_reg_type = Some(RegisterType::FloatingPoint);
                } else {
                    panic!("Unexpected section header: {section_header}");
                }
                let separator = lines.next().unwrap();
                assert!(separator.starts_with('-'));
                consume_empty_lines(&mut lines);
                parse_state = ParseState::TableStart;
            }
            ParseState::TableStart => {
                let top = lines.next().unwrap();
                assert!(top.starts_with('+'));
                let column_headers = lines.next().unwrap();
                assert!(column_headers
                    .eq("|Register | ABI Name | Description                       | Saver  |"));
                parse_state = ParseState::TableSeparator;
            }
            ParseState::TableSeparator => {
                let separator = lines.next().unwrap();
                assert!(separator.starts_with('+'));
                match lines.peek() {
                    Some(next) => {
                        if next.is_empty() {
                            parse_state = ParseState::TableEnd;
                        } else {
                            parse_state = ParseState::TableEntry;
                        }
                    }
                    None => parse_state = ParseState::TableEnd,
                }
            }
            ParseState::TableEntry => {
                let entries: Vec<&str> = lines
                    .next()
                    .unwrap()
                    .trim_start_matches('|')
                    .trim_end_matches('|')
                    .split('|')
                    .collect();
                assert!(entries.len() == 4);
                let saved_info = if entries[3].trim_ascii().is_empty() {
                    String::new()
                } else {
                    format!("\n{} saved", entries[3].trim_ascii())
                };
                let description = format!("{}{}", entries[2].trim_ascii(), saved_info);
                let reg_name = entries[0].trim_ascii().to_lowercase();
                let curr_register = Register {
                    name: reg_name,
                    description: Some(description),
                    reg_type: curr_reg_type,
                    arch: Some(Arch::RISCV),
                    ..Default::default()
                };
                registers.push(curr_register);
                parse_state = ParseState::TableSeparator;
            }
            ParseState::TableEnd => {
                consume_empty_lines(&mut lines);
                if lines.peek().is_some() {
                    parse_state = ParseState::SectionStart;
                } else {
                    parse_state = ParseState::FileEnd;
                }
            }
            ParseState::FileEnd => break,
        }
    }

    registers
}

/// Parse all of the RISCV instruction rst files inside of `docs_dir`
/// Each file is expected to correspond to part of an `Instruction` object
///
/// Current function assumes that the RST file is already read and that it's been given a reference
/// to its contents (`&str`).
///
/// # Errors
///
/// This function will return `Err` if an rst file within `docs_path` cannot be parsed,
/// or if `docs_path` cannot be read
///
/// # Panics
///
/// Will panic the parser fails to extract an instruction name from a given file
pub fn populate_riscv_instructions(docs_path: &PathBuf) -> Result<Vec<Instruction>> {
    let mut instructions_map = HashMap::<String, Instruction>::new();

    // ensure we iterate through all files in a deterministic order
    let mut entries: Vec<PathBuf> = std::fs::read_dir(docs_path)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    entries.sort();

    // parse all instruction docs
    for path in entries {
        if let Ok(docs) = std::fs::read_to_string(&path) {
            for instr in parse_riscv_instructions(&docs) {
                let instr_name = instr.name.to_ascii_lowercase();
                assert!(!instructions_map.contains_key(&instr_name));
                instructions_map.insert(instr_name, instr);
            }
        }
    }

    Ok(instructions_map.into_values().collect())
}

/// Parse an rst file containing the documentation for several RISCV instructions
///
/// # Errors
///
/// This function is highly specialized to parse a handful of files and will panic or return
/// `Err` for most mal-formed inputs
///
/// # Panics
///
/// This function is highly specialized to parse a handful of files and will panic or return
/// `Err` for most mal-formed/unexpected inputs
#[allow(clippy::too_many_lines)]
fn parse_riscv_instructions(rst_contents: &str) -> Vec<Instruction> {
    // We could pull in an actual rst parser to do this, but the files' contents
    // are straightforward/structured enough that this should be fairly trivial
    enum ParseState {
        FileStart,
        InstructionStart,
        InstructionTableInfo,
        InstructionFormat,
        InstructionDescription,
        InstructionImplementation,
        InstructionExpansion,
        FileEnd,
    }
    let mut parse_state = ParseState::FileStart;
    let mut instructions = Vec::new();
    let mut curr_instruction = Instruction {
        arch: Some(Arch::RISCV),
        ..Default::default()
    };
    let mut lines = rst_contents.lines().peekable();

    loop {
        match parse_state {
            ParseState::FileStart => {
                let _header = lines.next().unwrap();
                let separator = lines.next().unwrap();
                assert!(separator.trim_ascii().starts_with('='));
                consume_empty_lines(&mut lines);
                parse_state = ParseState::InstructionStart;
            }
            ParseState::InstructionStart => {
                curr_instruction.name = lines.next().unwrap().trim_ascii().to_ascii_lowercase();
                let separator = lines.next().unwrap();
                // e.g. ----------
                assert!(separator.trim_ascii().starts_with('-'));
                consume_empty_lines(&mut lines);

                // some forms have an explanation for the mnemonic before the table section
                if !lines.peek().unwrap().starts_with("..") {
                    curr_instruction.summary =
                        format!("{}\n\n", lines.next().unwrap().trim_ascii());
                    consume_empty_lines(&mut lines);
                }
                parse_state = ParseState::InstructionTableInfo;
            }
            ParseState::InstructionTableInfo => {
                // e.g. .. tabularcolumns:: |c|c|c|c|c|c|c|c|
                let table_info_1 = lines.next().unwrap();
                assert!(table_info_1.trim_ascii().starts_with(".."));
                // e.g. .. table::
                let table_info_2 = lines.next().unwrap();
                assert!(table_info_2.trim_ascii().starts_with(".."));

                consume_empty_lines(&mut lines);

                /* e.g.
                  +-----+--+--+-----+-----+-----+-----+-----+---+
                  |31-27|26|25|24-20|19-15|14-12|11-7 |6-2  |1-0|
                  +-----+--+--+-----+-----+-----+-----+-----+---+
                  |11100|aq|rl|rs2  |rs1  |011  |rd   |01011|11 |
                  +-----+--+--+-----+-----+-----+-----+-----+---+
                */
                let top = lines.next().unwrap();
                assert!(top.trim_ascii().starts_with('+'));
                let first_row = lines.next().unwrap();
                assert!(first_row.trim_ascii().starts_with('|'));
                let middle = lines.next().unwrap();
                assert!(middle.trim_ascii().starts_with('+'));
                let second_row = lines.next().unwrap();
                assert!(second_row.trim_ascii().starts_with('|'));
                let bottom = lines.next().unwrap();
                assert!(bottom.trim_ascii().starts_with('+'));
                consume_empty_lines(&mut lines);
                parse_state = ParseState::InstructionFormat;
            }
            ParseState::InstructionFormat => {
                let header = lines.next().unwrap();
                assert!(header.eq(":Format:"));
                curr_instruction.asm_templates.push(
                    lines
                        .next()
                        .unwrap()
                        .trim_ascii()
                        .trim_start_matches('|')
                        .trim_ascii()
                        .to_string(),
                );
                consume_empty_lines(&mut lines);
                parse_state = ParseState::InstructionDescription;
            }
            ParseState::InstructionDescription => {
                let header = lines.next().unwrap();
                assert!(header.eq(":Description:"));
                while let Some(next) = lines.peek() {
                    if next.contains('|') {
                        curr_instruction.summary += lines
                            .next()
                            .unwrap()
                            .trim_ascii()
                            .trim_start_matches('|')
                            .trim_ascii();
                    } else {
                        break;
                    }
                }
                consume_empty_lines(&mut lines);
                parse_state = ParseState::InstructionImplementation;
            }
            ParseState::InstructionImplementation => {
                let header = lines.next().unwrap();
                assert!(header.eq(":Implementation:"));
                let _impl_body = lines.next(); // e.g. x[rd] = AMO64(M[x[rs1]] MAXU x[rs2])
                consume_empty_lines(&mut lines);
                parse_state = ParseState::InstructionExpansion;
            }
            // NOTE: This field isn't present in most files
            ParseState::InstructionExpansion => {
                match lines.peek() {
                    Some(&":Expansion:") => {
                        let header = lines.next().unwrap();
                        assert!(header.eq(":Expansion:"));
                        let _exp_body = lines.next(); // e.g. lw rd\',offset[6:2](rs1\')
                        consume_empty_lines(&mut lines);
                        if lines.peek().is_some() {
                            parse_state = ParseState::InstructionStart;
                        } else {
                            parse_state = ParseState::FileEnd;
                        }
                    }
                    Some(other) => {
                        if other.eq(&".. [classify table]") {
                            consume_classify_table(&mut lines);
                        }
                        if lines.peek().is_some() {
                            parse_state = ParseState::InstructionStart;
                        } else {
                            parse_state = ParseState::FileEnd;
                        }
                    }
                    None => parse_state = ParseState::FileEnd,
                }

                instructions.push(curr_instruction);
                curr_instruction = Instruction {
                    arch: Some(Arch::RISCV),
                    ..Default::default()
                };
            }
            ParseState::FileEnd => break,
        }
    }

    instructions
}

fn consume_empty_lines(line_iter: &mut Peekable<Lines>) {
    while let Some(next) = line_iter.peek() {
        if next.is_empty() {
            _ = line_iter.next();
        } else {
            break;
        }
    }
}

fn consume_classify_table(line_iter: &mut Peekable<Lines>) {
    let info_1 = line_iter.next().unwrap();
    assert!(info_1.eq(".. [classify table]"));
    let info_2 = line_iter.next().unwrap();
    assert!(info_2.eq(".. table::"));
    let info_3 = line_iter.next().unwrap();
    assert!(info_3.trim_ascii().eq("Classify Table:"));
    let empty = line_iter.next().unwrap();
    assert!(empty.is_empty());
    while let Some(next) = line_iter.peek() {
        if next.is_empty() {
            break;
        }
        _ = line_iter.next();
    }
}

/// Parse all of the ARM instruction xml files inside of `docs_dir`
/// Each file is expected to correspond to part of an `Instruction` object
///
/// Current function assumes that the XML file is already read and that it's been given a reference
/// to its contents (`&str`).
///
/// # Errors
///
/// This function will return `Err` if an xml file within `docs_path` cannot be parsed,
/// or if `docs_path` cannot be read
///
/// # Panics
///
/// Will panic the parser fails to extract an instruction name from a given file
pub fn populate_arm_instructions(docs_path: &PathBuf) -> Result<Vec<Instruction>> {
    let mut instructions_map = HashMap::<String, Instruction>::new();
    let mut alias_map = HashMap::<String, Vec<InstructionAlias>>::new();

    // ensure we iterate through all files in a deterministic order
    let mut entries: Vec<PathBuf> = std::fs::read_dir(docs_path)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    entries.sort();

    // parse all instruction and instruction alias docs
    for path in entries {
        if path.extension().unwrap_or_default() != "xml"
            || path.file_stem().unwrap_or_default() == "notice"
            || path.file_stem().unwrap_or_default() == "constraint_text_mappings"
            || path.file_stem().unwrap_or_default() == "shared_pseudocode"
        {
            continue;
        }
        if let Ok(docs) = std::fs::read_to_string(&path) {
            if let Some((alias, aliased_instr)) = parse_arm_alias(&docs)? {
                assert!(!aliased_instr.is_empty());
                let aliases = alias_map.entry(aliased_instr).or_default();
                aliases.push(alias);
            } else if let Some(mut instr) = parse_arm_instruction(&docs) {
                assert!(!instr.name.is_empty());
                if let Some(entry) = instructions_map.get_mut(&instr.name) {
                    entry.aliases.append(&mut instr.aliases);
                    entry.asm_templates.append(&mut instr.asm_templates);
                    if entry.summary.is_empty() {
                        entry.summary = instr.summary;
                    }
                } else {
                    instructions_map.insert(instr.name.clone(), instr);
                }
            }
        } else {
            println!(
                "WARNING: Skipping entry, could not read file {}",
                path.display()
            );
        }
    }

    // add aliases to their corresponding instruction, creating them as necessary
    for (instr_name, aliases) in &mut alias_map {
        if let Some(entry) = instructions_map.get_mut(instr_name) {
            entry.aliases.append(aliases);
        } else {
            instructions_map.insert(
                instr_name.to_owned(),
                Instruction {
                    name: instr_name.to_owned(),
                    // TODO:currently changing into either doesn't change
                    // anything as both source form the 64bit info which should
                    // change when arm32 info is added
                    arch: Some(Arch::ARM64),
                    aliases: aliases.to_owned(),
                    ..Default::default()
                },
            );
        }
    }

    Ok(instructions_map.into_values().collect())
}

/// Parse an xml file containing the documentation for a single ARM instruction
/// Treats the contents as an instruction alias, and returns `None` if it is not
///
/// # Errors
///
/// This function is highly specialized to parse a handful of files and will panic or return
/// `Err` for most mal-formed inputs
///
/// # Panics
///
/// This function is highly specialized to parse a handful of files and will panic or return
/// `Err` for most mal-formed/unexpected inputs
fn parse_arm_alias(xml_contents: &str) -> Result<Option<(InstructionAlias, String)>> {
    // iterate through the XML
    let mut reader = Reader::from_str(xml_contents);
    let mut aliased_instr: Option<String> = None;
    let mut alias = InstructionAlias::default();
    let mut curr_template: Option<String> = None;
    let mut in_desc = false;
    let mut in_para = false;
    let mut in_template = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.name() {
                QName(b"instructionsection") => {
                    for attr in e.attributes() {
                        let Attribute { key, value } = attr.unwrap();
                        if b"title" == key.into_inner() {
                            alias.title = ustr::get_str(&value).to_string();
                        }
                    }
                }
                QName(b"desc") => in_desc = true,
                QName(b"para") => in_para = true,
                QName(b"asmtemplate") => in_template = true,
                QName(b"alphaindex" | b"encodingindex") => return Ok(None),
                _ => {}
            },
            Ok(Event::Text(ref txt)) => {
                if in_template {
                    let cleaned = txt.unescape().unwrap();
                    if let Some(existing) = curr_template {
                        curr_template = Some(format!("{existing}{cleaned}"));
                    } else {
                        let mut new_template = cleaned.into_owned().trim_ascii().to_owned();
                        new_template.push(' ');
                        curr_template = Some(new_template);
                    }
                } else if in_desc && in_para && alias.summary.is_empty() {
                    ustr::get_str(txt).clone_into(&mut alias.summary);
                }
            }
            Ok(Event::Empty(ref e)) => {
                if QName(b"docvar") == e.name() {
                    let mut alias_next = false;
                    for attr in e.attributes() {
                        let Attribute { key, value } = attr.unwrap();
                        // TODO: we can get the correct alias from the id of an alias mnemonic
                        // else the actual alias is the last docvar in the docvars tag
                        if alias_next && b"value" == key.into_inner() {
                            aliased_instr = Some(ustr::get_str(&value).to_ascii_lowercase());
                            break;
                        }
                        if b"key" == key.into_inner()
                            && b"alias_mnemonic" == ustr::get_str(&value).as_bytes()
                        {
                            alias_next = true;
                        }
                    }
                }
            }
            // end event
            Ok(Event::End(ref e)) => match e.name() {
                QName(b"instructionsection") => break,
                QName(b"asmtemplate") => {
                    if let Some(template) = curr_template.take() {
                        alias.asm_templates.push(template);
                    }
                    in_template = false;
                }
                QName(b"docvars") => {
                    if aliased_instr.is_none() {
                        return Ok(None);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    aliased_instr.map_or_else(|| Ok(None), |aliased_name| Ok(Some((alias, aliased_name))))
}

/// Parse an xml file containing the documentation for a single ARM instruction
///
/// # Errors
///
/// This function is highly specialized to parse a handful of files and will panic or return
/// `Err` for most mal-formed inputs
///
/// # Panics
///
/// This function is highly specialized to parse a handful of files and will panic or return
/// `Err` for most mal-formed/unexpected inputs
fn parse_arm_instruction(xml_contents: &str) -> Option<Instruction> {
    // iterate through the XML
    let mut reader = Reader::from_str(xml_contents);

    // ref to the instruction that's currently under construction
    let mut instruction = Instruction {
        // TODO: switch for archs
        arch: Some(Arch::ARM64),
        ..Default::default()
    };
    let mut curr_template: Option<String> = None;
    let mut in_desc = false;
    let mut in_para = false;
    let mut in_template = false;

    debug!("Parsing instruction XML contents...");
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.name() {
                QName(b"desc") => in_desc = true,
                QName(b"para") => in_para = true,
                QName(b"asmtemplate") => in_template = true,
                QName(b"alphaindex" | b"encodingindex") => return None,
                _ => {}
            },
            Ok(Event::Empty(ref e)) => {
                // e.g. <docvar key="mnemonic" value="ABS"/>
                if QName(b"docvar") == e.name() {
                    // There are multiple entries like this in each opcode file, but
                    // *all* of them are the same within each file, so it doesn't matter which
                    // one we use
                    if instruction.name.is_empty() {
                        let mut mnemonic_next = false;
                        for attr in e.attributes() {
                            let Attribute { key: _, value } = attr.unwrap();
                            if b"mnemonic" == ustr::get_str(&value).as_bytes() {
                                mnemonic_next = true;
                            } else if mnemonic_next {
                                instruction.name = ustr::get_str(&value).to_ascii_lowercase();
                                break;
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(ref txt)) => {
                if in_template {
                    let cleaned = txt.unescape().unwrap();
                    if let Some(existing) = curr_template {
                        curr_template = Some(format!("{existing}{cleaned}"));
                    } else {
                        let mut new_template = cleaned.into_owned().trim_ascii().to_owned();
                        new_template.push(' ');
                        curr_template = Some(new_template);
                    }
                } else if in_desc && in_para && instruction.summary.is_empty() {
                    ustr::get_str(txt).clone_into(&mut instruction.summary);
                }
            }
            // end event
            Ok(Event::End(ref e)) => {
                match e.name() {
                    QName(b"instructionsection") => break,
                    QName(b"encoding") => {
                        if let Some(template) = curr_template.take() {
                            instruction.asm_templates.push(template);
                        }
                    }
                    QName(b"desc") => in_desc = false,
                    QName(b"para") => in_para = false,
                    QName(b"asmtemplate") => in_template = false,
                    _ => {} // unknown event
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => {} // rest of events that we don't consider
        }
    }

    Some(instruction)
}

/// Parse the provided XML contents and return a vector of all the instructions based on that.
/// If parsing fails, the appropriate error will be returned instead.
///
/// Current function assumes that the XML file is already read and that it's been given a reference
/// to its contents (`&str`).
///
/// # Errors
///
/// This function is highly specialized to parse a handful of files and will panic or return
/// `Err` for most mal-formed inputs
///
/// # Panics
///
/// This function is highly specialized to parse a handful of files and will panic or return
/// `Err` for most mal-formed/unexpected inputs
pub fn populate_instructions(xml_contents: &str) -> Result<Vec<Instruction>> {
    // initialise the instruction set
    let mut instructions_map = HashMap::<String, Instruction>::new();

    // iterate through the XML
    let mut reader = Reader::from_str(xml_contents);

    // ref to the instruction that's currently under construction
    let mut curr_instruction = Instruction::default();
    let mut curr_instruction_form = InstructionForm::default();
    let mut arch: Option<Arch> = None;

    debug!("Parsing instruction XML contents...");
    loop {
        match reader.read_event() {
            // start event
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    QName(b"InstructionSet") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if b"name" == key.into_inner() {
                                arch = Arch::from_str(ustr::get_str(&value)).ok();
                            } else {
                                warn!("Failed to parse architecture name");
                            }
                        }
                    }
                    QName(b"Instruction") => {
                        // start of a new instruction
                        curr_instruction = Instruction::default();
                        curr_instruction.arch = arch;

                        // iterate over the attributes
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            match ustr::get_str(key.into_inner()) {
                                "name" => {
                                    let name = ustr::get_str(&value);
                                    curr_instruction.name = name.to_ascii_lowercase();
                                }
                                "summary" => {
                                    ustr::get_str(&value).clone_into(&mut curr_instruction.summary);
                                }
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

                        // new instruction form
                        curr_instruction_form = InstructionForm::default();

                        // iterate over the attributes
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            match ustr::get_str(key.into_inner()) {
                                "gas-name" => {
                                    curr_instruction_form.gas_name =
                                        Some(ustr::get_str(&value).to_owned());
                                }
                                "go-name" => {
                                    curr_instruction_form.go_name =
                                        Some(ustr::get_str(&value).to_owned());
                                }
                                "mmx-mode" => {
                                    let value_ = value.as_ref();
                                    curr_instruction_form.mmx_mode =
                                        Some(MMXMode::from_str(ustr::get_str(value_))?);
                                }
                                "xmm-mode" => {
                                    let value_ = value.as_ref();
                                    curr_instruction_form.xmm_mode =
                                        Some(XMMMode::from_str(ustr::get_str(value_))?);
                                }
                                "cancelling-inputs" => match ustr::get_str(&value) {
                                    "true" => curr_instruction_form.cancelling_inputs = Some(true),
                                    "false" => {
                                        curr_instruction_form.cancelling_inputs = Some(false);
                                    }
                                    val => {
                                        return Err(anyhow!(
                                            "Unknown value '{val}' for XML attribute cancelling inputs"
                                        ));
                                    }
                                },
                                "nacl-version" => {
                                    curr_instruction_form.nacl_version =
                                        value.as_ref().first().copied();
                                }
                                "nacl-zero-extends-outputs" => match ustr::get_str(&value) {
                                    "true" => {
                                        curr_instruction_form.nacl_zero_extends_outputs =
                                            Some(true);
                                    }
                                    "false" => {
                                        curr_instruction_form.nacl_zero_extends_outputs =
                                            Some(false);
                                    }
                                    val => {
                                        return Err(anyhow!(
                                                "Unknown value '{val}' for XML attribute nacl-zero-extends-outputs",
                                            ));
                                    }
                                },
                                "z80name" => {
                                    curr_instruction_form.z80_name =
                                        Some(ustr::get_str(&value).to_owned());
                                }
                                "form" => {
                                    let value_ = ustr::get_str(&value);
                                    curr_instruction_form.urls.push(format!(
                                        "https://www.zilog.com/docs/z80/z80cpu_um.pdf#{}",
                                        encode_www_form_urlencoded(value_)
                                    ));
                                    curr_instruction_form.z80_form = Some(value_.to_string());
                                }
                                _ => {}
                            }
                        }
                    }
                    // TODO
                    QName(b"Encoding") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if key.into_inner() == b"byte" {
                                let disp_code = ustr::get_str(&value);
                                if let Some(ref mut opcodes) = curr_instruction_form.z80_opcode {
                                    opcodes.push_str(disp_code);
                                } else {
                                    curr_instruction_form.z80_opcode = Some(disp_code.to_owned());
                                }
                            }
                        }
                    }
                    _ => {} // unknown event
                }
            }
            Ok(Event::Empty(ref e)) => {
                match e.name() {
                    QName(b"ISA") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if key.into_inner() == b"id" {
                                {
                                    curr_instruction_form.isa = Some(
                                        ISA::from_str(ustr::get_str(value.as_ref()))
                                            .unwrap_or_else(|_| {
                                                panic!(
                                                    "Unexpected ISA variant {}",
                                                    ustr::get_str(&value)
                                                )
                                            }),
                                    );
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
                            match key.into_inner() {
                                b"type" => {
                                    type_ = match OperandType::from_str(ustr::get_str(&value)) {
                                        Ok(op_type) => op_type,
                                        Err(_) => {
                                            return Err(anyhow!(
                                                "Unknown value for operand type -- Variant: {}",
                                                ustr::get_str(&value)
                                            ));
                                        }
                                    }
                                }
                                b"input" => match value.as_ref() {
                                    b"true" => input = Some(true),
                                    b"false" => input = Some(false),
                                    _ => return Err(anyhow!("Unknown value for operand type")),
                                },
                                b"output" => match value.as_ref() {
                                    b"true" => output = Some(true),
                                    b"false" => output = Some(false),
                                    _ => return Err(anyhow!("Unknown value for operand type")),
                                },
                                b"extended-size" => {
                                    extended_size =
                                        Some(ustr::get_str(value.as_ref()).parse::<usize>()?);
                                }
                                _ => {} // unknown event
                            }
                        }

                        curr_instruction_form.operands.push(Operand {
                            type_,
                            input,
                            output,
                            extended_size,
                        });
                    }
                    QName(b"TimingZ80") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if key.into_inner() == b"value" {
                                let z80 = match Z80TimingInfo::from_str(ustr::get_str(&value)) {
                                    Ok(timing) => timing,
                                    Err(e) => return Err(anyhow!(e)),
                                };
                                if let Some(ref mut timing_entry) = curr_instruction_form.z80_timing
                                {
                                    timing_entry.z80 = z80;
                                } else {
                                    curr_instruction_form.z80_timing = Some(Z80Timing {
                                        z80,
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                    QName(b"TimingZ80M1") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if key.into_inner() == b"value" {
                                let z80_plus_m1 =
                                    match Z80TimingInfo::from_str(ustr::get_str(&value)) {
                                        Ok(timing) => timing,
                                        Err(e) => return Err(anyhow!(e)),
                                    };
                                if let Some(ref mut timing_entry) = curr_instruction_form.z80_timing
                                {
                                    timing_entry.z80_plus_m1 = z80_plus_m1;
                                } else {
                                    curr_instruction_form.z80_timing = Some(Z80Timing {
                                        z80_plus_m1,
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                    QName(b"TimingR800") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if key.into_inner() == b"value" {
                                let r800 = match Z80TimingInfo::from_str(ustr::get_str(&value)) {
                                    Ok(timing) => timing,
                                    Err(e) => return Err(anyhow!(e)),
                                };
                                if let Some(ref mut timing_entry) = curr_instruction_form.z80_timing
                                {
                                    timing_entry.r800 = r800;
                                } else {
                                    curr_instruction_form.z80_timing = Some(Z80Timing {
                                        r800,
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                    QName(b"TimingR800Wait") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if key.into_inner() == b"value" {
                                let r800_plus_wait =
                                    match Z80TimingInfo::from_str(ustr::get_str(&value)) {
                                        Ok(timing) => timing,
                                        Err(e) => return Err(anyhow!(e)),
                                    };
                                if let Some(ref mut timing_entry) = curr_instruction_form.z80_timing
                                {
                                    timing_entry.r800_plus_wait = r800_plus_wait;
                                } else {
                                    curr_instruction_form.z80_timing = Some(Z80Timing {
                                        r800_plus_wait,
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                    _ => {} // unknown event
                }
            }
            // end event
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
                    _ => {} // unknown event
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => {} // rest of events that we don't consider
        }
    }

    if let Some(Arch::X86 | Arch::X86_64) = arch {
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
            if let Some(instruction) = instructions_map.get_mut(instruction_name) {
                instruction.url = Some(x86_online_docs.clone() + url_suffix);
            }
        }
    }

    Ok(instructions_map.into_values().collect())
}

pub fn populate_name_to_instruction_map<'instruction>(
    arch: Arch,
    instructions: &'instruction Vec<Instruction>,
    names_to_instructions: &mut NameToInstructionMap<'instruction>,
) {
    // Add the "true" names first
    for instruction in instructions {
        for name in &instruction.get_primary_names() {
            names_to_instructions.insert((arch, name), instruction);
        }
    }
    // Add alternate form names next, ensuring we don't overwrite existing entries
    for instruction in instructions {
        for name in &instruction.get_associated_names() {
            names_to_instructions
                .entry((arch, name))
                .or_insert_with(|| instruction);
        }
    }
}

/// Parse the provided XML contents and return a vector of all the registers based on that.
/// If parsing fails, the appropriate error will be returned instead.
///
/// Current function assumes that the XML file is already read and that it's been given a reference
/// to its contents (`&str`).
///
/// # Errors
///
/// This function is highly specialized to parse a handful of files and will panic or return
/// `Err` for most mal-formed/unexpected inputs
///
/// # Panics
///
/// This function is highly specialized to parse a handful of files and will panic or return
/// `Err` for most mal-formed/unexpected inputs
pub fn populate_registers(xml_contents: &str) -> Result<Vec<Register>> {
    let mut registers_map = HashMap::<String, Register>::new();

    // iterate through the XML
    let mut reader = Reader::from_str(xml_contents);

    // ref to the register that's currently under construction
    let mut curr_register = Register::default();
    let mut curr_bit_flag = RegisterBitInfo::default();
    let mut arch: Option<Arch> = None;

    debug!("Parsing register XML contents...");
    loop {
        match reader.read_event() {
            // start event
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    QName(b"InstructionSet") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if b"name" == key.into_inner() {
                                arch = Arch::from_str(ustr::get_str(&value)).ok();
                            }
                        }
                    }
                    QName(b"Register") => {
                        // start of a new register
                        curr_register = Register::default();
                        curr_register.arch = arch;

                        // iterate over the attributes
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            match key.into_inner() {
                                b"name" => {
                                    let name_ = String::from(ustr::get_str(&value));
                                    curr_register.name = name_.to_ascii_lowercase();
                                }
                                b"description" => {
                                    curr_register.description =
                                        Some(String::from(ustr::get_str(&value)));
                                }
                                b"type" => {
                                    curr_register.reg_type =
                                        RegisterType::from_str(ustr::get_str(&value))
                                            .map_or(None, |reg| Some(reg));
                                }
                                b"width" => {
                                    curr_register.width =
                                        RegisterWidth::from_str(ustr::get_str(&value))
                                            .map_or(None, |width| Some(width));
                                }
                                _ => {}
                            }
                        }
                    }
                    // Actual flag bit info
                    QName(b"Flag") => {
                        curr_bit_flag = RegisterBitInfo::default();

                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            match key.into_inner() {
                                b"bit" => {
                                    curr_bit_flag.bit =
                                        ustr::get_str(&value).parse::<u32>().unwrap();
                                }
                                b"label" => {
                                    curr_bit_flag.label = String::from(ustr::get_str(&value));
                                }
                                b"description" => {
                                    curr_bit_flag.description = String::from(ustr::get_str(&value));
                                }
                                b"pae" => {
                                    curr_bit_flag.pae = String::from(ustr::get_str(&value));
                                }
                                b"longmode" => {
                                    curr_bit_flag.long_mode = String::from(ustr::get_str(&value));
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {} // unknown event
                }
            }
            // end event
            Ok(Event::End(ref e)) => {
                match e.name() {
                    QName(b"Register") => {
                        // finish register
                        registers_map.insert(curr_register.name.clone(), curr_register.clone());
                    }
                    QName(b"Flag") => {
                        curr_register.push_flag(curr_bit_flag.clone());
                    }
                    _ => {} // unknown event
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => {} // rest of events that we don't consider
        }
    }

    // TODO: Add to URL fields here for x86/x86-64?
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

/// Parse the provided XML contents and return a vector of all the directives based on that.
/// If parsing fails, the appropriate error will be returned instead.
///
/// Current function assumes that the XML file is already read and that it's been given a reference
/// to its contents (`&str`).
///
/// # Errors
///
/// This function is highly specialized to parse a handful of files and will panic or return
/// `Err` for most mal-formed/unexpected inputs
///
/// # Panics
///
/// This function is highly specialized to parse a handful of files and will panic or return
/// `Err` for most mal-formed/unexpected inputs
pub fn populate_masm_nasm_directives(xml_contents: &str) -> Result<Vec<Directive>> {
    let mut directives_map = HashMap::<String, Directive>::new();

    // iterate through the XML
    let mut reader = Reader::from_str(xml_contents);

    // ref to the assembler directive that's currently under construction
    let mut curr_directive = Directive::default();
    let mut in_desc = false;

    debug!("Parsing directive XML contents...");
    loop {
        match reader.read_event() {
            // start event
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    QName(b"directive") => {
                        // start of a new directive
                        curr_directive = Directive::default();

                        // iterate over the attributes
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            match key.into_inner() {
                                b"name" => {
                                    let name = ustr::get_str(&value);
                                    curr_directive.name = name.to_ascii_lowercase();
                                }
                                b"tool" => {
                                    let assembler = Assembler::from_str(ustr::get_str(&value))?;
                                    curr_directive.assembler = Some(assembler);
                                }
                                _ => {}
                            }
                        }
                    }
                    QName(b"description") => {
                        in_desc = true;
                    }
                    _ => {} // unknown event
                }
            }
            Ok(Event::Text(ref txt)) => {
                if in_desc {
                    ustr::get_str(txt)
                        .trim_ascii()
                        .clone_into(&mut curr_directive.description);
                }
            }
            // end event
            Ok(Event::End(ref e)) => {
                if QName(b"directive") == e.name() {
                    directives_map.insert(curr_directive.name.clone(), curr_directive.clone());
                } else if QName(b"description") == e.name() {
                    in_desc = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => {} // rest of events that we don't consider
        }
    }

    // Since directive entries have their assembler labeled on a per-instance basis,
    // we check to make sure all of them have been assigned correctly
    for directive in directives_map.values() {
        assert!(directive.assembler.is_some());
    }

    Ok(directives_map.into_values().collect())
}
/// Parse the provided XML contents and return a vector of all the directives based on that.
/// If parsing fails, the appropriate error will be returned instead.
///
/// Current function assumes that the XML file is already read and that it's been given a reference
/// to its contents (`&str`).
///
/// # Errors
///
/// This function is highly specialized to parse a handful of files and will panic or return
/// `Err` for most mal-formed/unexpected inputs
///
/// # Panics
///
/// This function is highly specialized to parse a handful of files and will panic or return
/// `Err` for most mal-formed/unexpected inputs
pub fn populate_gas_directives(xml_contents: &str) -> Result<Vec<Directive>> {
    let mut directives_map = HashMap::<String, Directive>::new();

    // iterate through the XML
    let mut reader = Reader::from_str(xml_contents);

    // ref to the assembler directive that's currently under construction
    let mut curr_directive = Directive::default();
    let mut assembler: Option<Assembler> = None;

    debug!("Parsing directive XML contents...");
    loop {
        match reader.read_event() {
            // start event
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    QName(b"Assembler") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if b"name" == key.into_inner() {
                                assembler = Assembler::from_str(ustr::get_str(&value)).ok();
                            }
                        }
                    }
                    QName(b"Directive") => {
                        // start of a new directive
                        curr_directive = Directive::default();
                        curr_directive.assembler = assembler;

                        // iterate over the attributes
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            match key.into_inner() {
                                b"name" => {
                                    let name = ustr::get_str(&value);
                                    curr_directive.name = name.to_ascii_lowercase();
                                }
                                b"md_description" => {
                                    let description = ustr::get_str(&value);
                                    curr_directive.description =
                                        unescape(description).unwrap().to_string();
                                }
                                b"deprecated" => {
                                    curr_directive.deprecated =
                                        FromStr::from_str(ustr::get_str(&value)).unwrap();
                                }
                                b"url_fragment" => {
                                    curr_directive.url = Some(format!(
                                        "https://sourceware.org/binutils/docs-2.41/as/{}.html",
                                        ustr::get_str(&value)
                                    ));
                                }
                                _ => {}
                            }
                        }
                    }
                    QName(b"Signature") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if b"sig" == key.into_inner() {
                                let sig = ustr::get_str(&value);
                                curr_directive
                                    .signatures
                                    .push(unescape(sig).unwrap().to_string());
                            }
                        }
                    }
                    _ => {} // unknown event
                }
            }
            // end event
            Ok(Event::End(ref e)) => {
                if QName(b"Directive") == e.name() {
                    // finish directive
                    directives_map.insert(curr_directive.name.clone(), curr_directive.clone());
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => {} // rest of events that we don't consider
        }
    }

    Ok(directives_map.into_values().collect())
}

pub fn populate_name_to_directive_map<'directive>(
    assem: Assembler,
    directives: &'directive Vec<Directive>,
    names_to_directives: &mut NameToDirectiveMap<'directive>,
) {
    for directive in directives {
        for name in &directive.get_associated_names() {
            names_to_directives.insert((assem, name), directive);
        }
    }
}

fn get_docs_body(x86_online_docs: &str) -> Option<String> {
    // provide a URL example page
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
                error!("Failed to fetch documentation from {x86_online_docs} - Error: {e}.");
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

/// Searches for the asm-lsp cache directory.
///
/// - First checks for the `ASM_LSP_CACHE_DIR` environment variable. If this variable
///   is present and points to a valid directory, this path is returned.
/// - Otherwise, the function returns `~/.config/asm-lsp/`
///
/// # Errors
///
/// Returns `Err` if no directory can be found through `ASM_LSP_CACHE_DIR`, and
/// then no home directory can be found on the system
pub fn get_cache_dir() -> Result<PathBuf> {
    // first check if the appropriate environment variable is set
    if let Ok(path) = std::env::var("ASM_LSP_CACHE_DIR") {
        let path = PathBuf::from(path);
        // ensure the path is valid
        if path.is_dir() {
            return Ok(path);
        }
    }

    // If the environment variable isn't set or gives an invalid path, grab the home directory and build off of that
    let mut x86_cache_path = home::home_dir().ok_or_else(|| anyhow!("Home directory not found"))?;

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

fn get_x86_docs_web(x86_online_docs: &str) -> Result<String> {
    info!("Fetching further documentation from the web -> {x86_online_docs}...");
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

#[cfg(test)]
mod tests {
    use mockito::ServerOpts;

    use crate::parser::{get_cache_dir, populate_instructions};
    #[test]
    fn test_populate_instructions() {
        let mut server = mockito::Server::new_with_opts(ServerOpts {
            port: 8080,
            ..Default::default()
        });

        let _ = server
            .mock("GET", "/x86/")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(include_str!(
                "../docs_store/instr_info_cache/x86_instr_docs.html"
            ))
            .create();

        // Need to clear the cache file (if there is one)
        // to ensure a request is made for each test call
        let mut x86_cache_path = get_cache_dir().unwrap();
        x86_cache_path.push("x86_instr_docs.html");
        if x86_cache_path.is_file() {
            std::fs::remove_file(&x86_cache_path).unwrap();
        }
        let xml_conts_x86 = include_str!("../docs_store/opcodes/raw/x86.xml");
        assert!(populate_instructions(xml_conts_x86).is_ok());

        if x86_cache_path.is_file() {
            std::fs::remove_file(&x86_cache_path).unwrap();
        }
        let xml_conts_x86_64 = include_str!("../docs_store/opcodes/raw/x86_64.xml");
        assert!(populate_instructions(xml_conts_x86_64).is_ok());

        // Clean things up so we don't have an empty cache file
        if x86_cache_path.is_file() {
            std::fs::remove_file(&x86_cache_path).unwrap();
        }
    }
}
