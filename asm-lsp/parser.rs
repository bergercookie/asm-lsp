use std::{
    collections::HashMap,
    env::args,
    fs,
    io::Write as _,
    iter::Peekable,
    path::PathBuf,
    str::{FromStr, Lines},
};

use anyhow::{Result, anyhow};
use htmlentity::entity::ICodedDataTrait;
use quick_xml::Reader;
use quick_xml::escape::unescape;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use regex::Regex;
use reqwest;
use serde::Deserialize;
use url_escape::encode_www_form_urlencoded;

use crate::{
    AvrStatusRegister, AvrTiming, InstructionAlias,
    types::{
        Arch, Assembler, Directive, ISA, Instruction, InstructionForm, MMXMode, NameToDirectiveMap,
        NameToInstructionMap, NameToRegisterMap, Operand, OperandType, Register, RegisterBitInfo,
        RegisterType, RegisterWidth, XMMMode, Z80Timing, Z80TimingInfo,
    },
    ustr,
};

/// Parse all of the register information witin the documentation file
///
/// Current function assumes that the RST file is already read and that it's been given a reference
/// to its contents (`&str`).
///
/// # Errors
///
/// This function will not error, it maintains a `Result` return type for compatibility
/// with a macro in the server's test code
///
/// # Panics
///
/// This function is highly specialized to parse a specific file and will panic
/// for most mal-formed/unexpected inputs
pub fn populate_riscv_registers(rst_contents: &str) -> Result<Vec<Register>> {
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
                assert!(
                    column_headers
                        .eq("|Register | ABI Name | Description                       | Saver  |")
                );
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
                    arch: Arch::RISCV,
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

    Ok(registers)
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
        arch: Arch::RISCV,
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
                    arch: Arch::RISCV,
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
                    arch: Arch::ARM64,
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
        arch: Arch::ARM64,
        ..Default::default()
    };
    let mut curr_template: Option<String> = None;
    let mut in_desc = false;
    let mut in_para = false;
    let mut in_template = false;

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

/// Parse all of the MARS mips pseudo-ops from the `mips.txt` file
///
/// # Errors
///
/// This function will return `Err` if `contents` cannot be parsed
///
/// # Panics
///
/// This function is highly specialized to parse a single file and will panic if the file is not
/// in the expected format or if it contains unexpected content
///
/// <https://github.com/dpetersanderson/MARS/blob/main/PseudoOps.txt>
pub fn populate_mars_pseudo_instructions(contents: &str) -> Result<Vec<Instruction>> {
    let mut prev_instr: Option<&mut Instruction> = None;
    let mut instructions = Vec::new();

    for line in contents
        .lines()
        .filter(|l| !l.is_empty() && !l.trim_start().starts_with('#'))
        .map(str::trim)
    {
        let name = line.split_once(' ').unwrap().0;
        let (_, description) = line.split_once('#').unwrap();
        let template = line.replace('\t', " ");
        match prev_instr {
            Some(ref mut prev) if prev.name == name => {
                prev.asm_templates.push(template);
                continue;
            }
            _ => {}
        }

        let mut summary = description.trim().replace('\t', " ");
        if let Some(colon_idx) = summary.find(':') {
            // Only keep common description between psuedo op definitions
            summary = summary[..colon_idx].trim().to_string();
        }

        instructions.push(Instruction {
            name: name.to_string(),
            summary: format!("{summary}\n\nPseudo-op provided by the MARS assembler",),
            asm_templates: vec![template],
            arch: Arch::Mips,
            forms: Vec::new(),
            aliases: Vec::new(),
            url: None,
        });

        prev_instr = instructions.last_mut();
    }

    Ok(instructions)
}

/// Parse all of the mips instruction in the `mips.json` file
///
/// # Errors
///
/// This function will return `Err` if `json_contents` cannot be parsed
pub fn populate_mips_instructions(json_contents: &str) -> Result<Vec<Instruction>> {
    #[derive(Deserialize, Debug)]
    struct MipsInstruction {
        pub name: String,
        pub summary: String,
        pub asm_templates: Vec<String>,
    }

    impl From<MipsInstruction> for Instruction {
        fn from(instr: MipsInstruction) -> Self {
            Self {
                name: instr.name.to_ascii_lowercase(),
                summary: instr.summary,
                asm_templates: instr.asm_templates,
                arch: Arch::Mips,
                forms: Vec::new(),
                aliases: Vec::new(),
                url: Some(
                    "https://www.cs.cornell.edu/courses/cs3410/2008fa/MIPS_Vol2.pdf".to_string(),
                ),
            }
        }
    }

    let raw_instrs: Vec<MipsInstruction> =
        serde_json::from_str(json_contents).map_err(|e| anyhow!("Failed to parse JSON: {e}"))?;
    let instructions: Vec<Instruction> = raw_instrs.into_iter().map(Instruction::from).collect();

    Ok(instructions)
}

/// Parse the provided HTML contents and return a vector of all the instructions based on that.
/// <https://www.masswerk.at/6502/6502_instruction_set.html>
///
/// # Errors
///
/// This function is highly specialized to parse a single file and will panic or return
/// `Err` for most mal-formed inputs
///
/// # Panics
///
/// This function is highly specialized to parse a single file and will panic or return
/// `Err` for most mal-formed/unexpected inputs
// NOTE: We could use an HTML parsing library like scraper or html5ever, but the input
// is regular/constrained enough that we can just use some regexes and avoid
// the extra dependency
pub fn populate_6502_instructions(html_conts: &str) -> Result<Vec<Instruction>> {
    let name_regex = Regex::new(r#"<dt id="[A-Z]{3}">(?<name>[A-Z]{3})</dt>$"#).unwrap();
    let summary_regex = Regex::new(r#"<p aria-label="summary">(?<summary>.+)</p>$"#).unwrap();
    let mut instructions = Vec::new();
    let start = {
        let start_marker = r#"<dl class="opcodes">"#;
        let section_start = html_conts.find(start_marker).unwrap();
        section_start + start_marker.len() + 1 // + 1 for '\n'
    };
    let mut lines = html_conts[start..].lines().peekable();
    // opcode id
    while let Some(name_line) = lines.next() {
        if name_line.is_empty() {
            continue;
        }
        let name = &name_regex.captures(name_line).unwrap()["name"];
        assert_eq!(lines.next().unwrap(), "<dd>");
        // summary
        let mut summary =
            summary_regex.captures(lines.next().unwrap()).unwrap()["summary"].to_string();
        let implementation_notes_marker = r#"<p aria-label="notes on the implementation">"#;
        let synopsis_marker = r#"<p aria-label="synopsis">"#;
        if lines
            .peek()
            .unwrap()
            .starts_with(implementation_notes_marker)
        {
            summary.push('\n');
            while !lines.peek().unwrap().starts_with(synopsis_marker) {
                summary += &lines
                    .next()
                    .unwrap()
                    .replace(r#"<p aria-label="notes on the implementation">"#, "")
                    .replace("<br />", "")
                    .replace("</p>", "");
            }
        }
        // synopsis
        let synopsis_line = lines.next().unwrap();
        let mut synopsis = String::new();
        let mut prev_idx = 0;
        for (i, c) in synopsis_line.chars().enumerate() {
            match c {
                '<' => {
                    if prev_idx != 0 {
                        let bytes: Vec<u8> = synopsis_line.as_bytes()[prev_idx..i].to_vec();
                        let decoded = htmlentity::entity::decode(&bytes).to_string().unwrap();
                        synopsis += &decoded;
                    }
                }
                '>' => prev_idx = i + 1,
                _ => {}
            }
        }
        // flags
        assert_eq!(
            r#"<table aria-label="flags">"#,
            lines.next().unwrap().trim()
        );
        // This is always the same
        assert_eq!(
            r"<tr><th>N</th><th>Z</th><th>C</th><th>I</th><th>D</th><th>V</th></tr>",
            lines.next().unwrap().trim()
        );
        let flag_line = lines.next().unwrap().trim();
        let flags: String = if flag_line.contains("from stack") {
            "from stack".to_string()
        } else {
            flag_line
                .chars()
                .skip("<tr><td>".len())
                .step_by("</td><td>".len() + 1)
                .take(6) // N, Z, C, I, D, V
                .collect()
        };
        assert!(
            flags.len() == 6 || flags.eq("from stack"),
            "name: {name}, flagline: {flag_line}"
        );
        assert_eq!("</table>", lines.next().unwrap().trim());
        // details (table)
        assert_eq!(
            r#"<table aria-label="details">"#,
            lines.next().unwrap().trim()
        );
        let mut templates = Vec::new();
        assert_eq!(
            r"<tr><th>addressing</th><th>assembler</th><th>opc</th><th>bytes</th><th>cycles</th></tr>",
            lines.next().unwrap().trim()
        );
        loop {
            let next = lines.next().unwrap().trim();
            if next.eq("</table>") {
                break;
            }
            let template_marker = "</td><td>";
            let start_idx = next.find(template_marker).unwrap() + template_marker.len();
            let end_offset = next[start_idx..].find(template_marker).unwrap();
            templates.push(next[start_idx..start_idx + end_offset].to_string());
        }
        assert_eq!("</dd>", lines.next().unwrap().trim());
        let combined_summary = format!("{summary}\n{synopsis}\nNZCIDV\n`{flags}`");
        instructions.push(Instruction {
            name: name.to_lowercase(),
            summary: combined_summary,
            forms: Vec::new(),
            asm_templates: templates,
            aliases: Vec::new(),
            arch: Arch::MOS6502,
            url: Some(format!(
                "https://www.masswerk.at/6502/6502_instruction_set.html#{}",
                name.to_uppercase()
            )),
        });
        if name.eq("TYA") {
            break;
        }
    }

    Ok(instructions)
}

/// Parse the provided JSON contents and return a vector of all the instructions based on that.
/// <https://github.com/open-power-sdk/PowerISA/blob/main/ISA.json>
///
/// # Errors
///
/// This function is highly specialized to parse a single file and will panic or return
/// `Err` for most mal-formed inputs
///
/// # Panics
///
/// This function is highly specialized to parse a single file and will panic or return
/// `Err` for most mal-formed/unexpected inputs
// NOTE:
// Raw JSON file pruned via the command:
// ```
// jq ".instructions | map({mnemonics: .mnemonics | map(del(.intrinsics)), body})" power-isa.json
// ```
pub fn populate_power_isa_instructions(json_conts: &str) -> Result<Vec<Instruction>> {
    #[allow(non_camel_case_types, clippy::upper_case_acronyms)]
    #[derive(Deserialize, Debug, Copy, Clone)]
    enum PowerReleaseRepr {
        P1,
        P2,
        PPC,
        #[serde(rename = "v2.00")]
        v200,
        #[serde(rename = "v2.01")]
        v201,
        #[serde(rename = "v2.02")]
        v202,
        #[serde(rename = "v2.03")]
        v203,
        #[serde(rename = "v2.04")]
        v204,
        #[serde(rename = "v2.05")]
        v205,
        #[serde(rename = "v2.06")]
        v206,
        #[serde(rename = "v2.07")]
        v207,
        #[serde(rename = "v3.0")]
        v30,
        #[serde(rename = "v3.0B")]
        v30B,
        #[serde(rename = "v3.0C")]
        v30C,
        #[serde(rename = "v3.1")]
        v31,
        #[serde(rename = "v3.1B")]
        v31B,
    }

    impl PowerReleaseRepr {
        fn release_message(self) -> String {
            String::from(match self {
                Self::P1 => "Introduced in POWER Architecture",
                Self::P2 => "Introduced in POWER2 Architecture",
                Self::PPC => "Introduced in PowerPC Architecture prior to v2.00",
                Self::v200 => "Introduced in PowerPC Architecture Version 2.00",
                Self::v201 => "Introduced in PowerPC Architecture Version 2.01",
                Self::v202 => "Introduced in PowerPC Architecture Version 2.02",
                Self::v203 => "Introduced in Power ISA Version 2.03",
                Self::v204 => "Introduced in Power ISA Version 2.04",
                Self::v205 => "Introduced in Power ISA Version 2.05",
                Self::v206 => "Introduced in Power ISA Version 2.06",
                Self::v207 => "Introduced in Power ISA Version 2.07",
                Self::v30 => "Introduced in Power ISA Version 3.0",
                Self::v30B => "Introduced in Power ISA Version 3.0B",
                Self::v30C => "Introduced in Power ISA Version 3.0C",
                Self::v31 => "Introduced in Power ISA Version 3.1",
                Self::v31B => "Introduced in Power ISA Version 3.1B",
            })
        }
    }
    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    struct PowerConditionRepr {
        pub field: String,
        pub value: String,
    }
    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    struct PowerLayoutRepr {
        pub name: String,
        pub size: String,
    }
    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    struct PowerMnemonicRepr {
        pub name: String,
        pub form: String,
        pub mnemonic: String,
        pub operands: Vec<String>,
        pub conditions: Vec<PowerConditionRepr>,
        pub layout: Vec<PowerLayoutRepr>,
        pub release: PowerReleaseRepr,
    }
    #[derive(Deserialize, Debug)]
    struct PowerJsonRepr {
        pub mnemonics: Vec<PowerMnemonicRepr>,
        pub body: Vec<String>,
    }

    impl From<PowerJsonRepr> for Vec<Instruction> {
        fn from(value: PowerJsonRepr) -> Self {
            let mut instructions = Self::new();
            for op in value.mnemonics {
                let name = op.mnemonic.trim();
                let mut instruction = Instruction {
                    arch: Arch::PowerISA,
                    name: name.to_string(),
                    ..Default::default()
                };
                instruction.summary = {
                    let operands = op.operands.iter().fold(String::new(), |accum, x| {
                        format!("{} + `{x}`", if accum.is_empty() { "" } else { "\n" })
                    });
                    let description = value.body.join("\n");

                    format!(
                        "\n{} ({})\n\n{operands}\n{description}",
                        op.name,
                        op.release.release_message(),
                    )
                };
                instructions.push(instruction);
            }

            instructions
        }
    }

    let json_instrs: Vec<PowerJsonRepr> = serde_json::from_str(json_conts)?;
    let mut instructions = Vec::new();
    for instr in json_instrs {
        instructions.append(&mut instr.into());
    }

    Ok(instructions)
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
    let mut arch: Arch = Arch::None;

    loop {
        match reader.read_event() {
            // start event
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    QName(b"InstructionSet") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if b"name" == key.into_inner() {
                                arch = Arch::from_str(ustr::get_str(&value)).unwrap_or_else(|e| {
                                    panic!("Failed parse Arch {} -- {e}", ustr::get_str(&value))
                                });
                            } else {
                                panic!("Failed to parse architecture name -- no name value");
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
                                curr_instruction_form.isa = Some(
                                    ISA::from_str(ustr::get_str(value.as_ref())).unwrap_or_else(
                                        |_| {
                                            panic!(
                                                "Unexpected ISA variant {}",
                                                ustr::get_str(&value)
                                            )
                                        },
                                    ),
                                );
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
                        assert!(curr_instruction.arch != Arch::None);
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

    if matches!(arch, Arch::X86 | Arch::X86_64) {
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

pub fn populate_name_to_instruction_map(
    arch: Arch,
    instructions: &Vec<Instruction>,
    names_to_instructions: &mut NameToInstructionMap,
) {
    for instruction in instructions {
        names_to_instructions.insert((arch, instruction.name.clone()), instruction.clone());
        // Inserts instruction form names in addition to the instruction's "main"
        // name
        for name in &instruction.get_associated_names() {
            names_to_instructions
                .entry((arch, (*name).to_string()))
                .or_insert_with(|| instruction.clone());
        }
    }
}

fn process_sreg_value(
    e: &BytesStart,
    curr_instruction_form: &mut InstructionForm,
    field_setter: impl FnOnce(&mut AvrStatusRegister, char),
) {
    for attr in e.attributes() {
        let Attribute { key, value } = attr.unwrap();
        if key.into_inner() == b"value" {
            let val = ustr::get_str(&value);
            let status = if val.eq("–") {
                '-'
            } else {
                ustr::get_str(&value)
                    .chars()
                    .next()
                    .expect("Empty status register value")
            };
            if let Some(ref mut sreg_entry) = curr_instruction_form.avr_status_register {
                field_setter(sreg_entry, status);
            } else {
                let mut sreg = AvrStatusRegister::default();
                field_setter(&mut sreg, status);
                curr_instruction_form.avr_status_register = Some(sreg);
            }
            break;
        }
    }
}

fn process_clock_value(
    e: &BytesStart,
    curr_instruction_form: &mut InstructionForm,
    field_setter: impl FnOnce(&mut AvrTiming, Option<String>),
) {
    for attr in e.attributes() {
        let Attribute { key, value } = attr.unwrap();
        if key.into_inner() == b"value" {
            let cycles = Some(ustr::get_str(&value).to_string());
            if let Some(ref mut timing_entry) = curr_instruction_form.avr_timing {
                field_setter(timing_entry, cycles);
            } else {
                let mut timing = AvrTiming::default();
                field_setter(&mut timing, cycles);
                curr_instruction_form.avr_timing = Some(timing);
            }
            break;
        }
    }
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
pub fn populate_avr_instructions(xml_contents: &str) -> Result<Vec<Instruction>> {
    // initialise the instruction set
    let mut instructions_map = HashMap::<String, Instruction>::new();

    // iterate through the XML
    let mut reader = Reader::from_str(xml_contents);

    // ref to the instruction that's currently under construction
    let mut curr_instruction = Instruction::default();
    let mut curr_instruction_form = InstructionForm::default();
    let mut arch: Arch = Arch::None;
    let mut curr_version: Option<String> = None;

    loop {
        match reader.read_event() {
            // start event
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    QName(b"InstructionSet") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if b"name" == key.into_inner() {
                                arch = Arch::from_str(ustr::get_str(&value)).unwrap_or_else(|e| {
                                    panic!("Failed parse Arch {} -- {e}", ustr::get_str(&value))
                                });
                                assert!(arch == Arch::Avr);
                            } else {
                                panic!("Failed parse Arch -- no name value");
                            }
                        }
                    }
                    QName(b"Instruction") => {
                        // start of a new instruction
                        curr_instruction = Instruction::default();
                        curr_instruction.arch = arch;

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
                    // Versions are defined a per-instruction form basis
                    QName(b"Version") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if "value" == ustr::get_str(key.into_inner()) {
                                curr_version = Some(ustr::get_str(&value).to_string());
                            }
                        }
                    }
                    QName(b"InstructionForm") => {
                        assert!(curr_version.is_some());
                        // new instruction form
                        curr_instruction_form = InstructionForm::default();
                        curr_instruction_form.avr_version.clone_from(&curr_version);

                        // iterate over the attributes
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            match ustr::get_str(key.into_inner()) {
                                "mnemonic" => {
                                    curr_instruction_form.avr_mneumonic =
                                        Some(ustr::get_str(&value).to_owned());
                                }
                                "summary" => {
                                    curr_instruction_form.avr_summary =
                                        Some(ustr::get_str(&value).to_owned());
                                }
                                _ => {}
                            }
                        }
                    }
                    // NOTE: Intentionally leaving out encoding nibbles unless that's desired...
                    // QName(b"Encoding") => {}
                    _ => {} // unknown event
                }
            }
            Ok(Event::Empty(ref e)) => {
                match e.name() {
                    QName(b"Operand") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if key.into_inner() == b"type" {
                                let val = ustr::get_str(&value);
                                for oper in val.split(',') {
                                    if oper.is_empty() {
                                        continue;
                                    }
                                    let Ok(type_) = OperandType::from_str(oper) else {
                                        return Err(anyhow!(
                                            "Unknown value for operand type -- Variant: {}",
                                            ustr::get_str(&value)
                                        ));
                                    };
                                    curr_instruction_form.operands.push(Operand {
                                        type_,
                                        input: None,
                                        output: None,
                                        extended_size: None,
                                    });
                                }
                            }
                        }
                    }
                    // Status register values
                    QName(b"I") => {
                        process_sreg_value(e, &mut curr_instruction_form, |sreg, val| sreg.i = val);
                    }
                    QName(b"T") => {
                        process_sreg_value(e, &mut curr_instruction_form, |sreg, val| sreg.t = val);
                    }
                    QName(b"H") => {
                        process_sreg_value(e, &mut curr_instruction_form, |sreg, val| sreg.h = val);
                    }
                    QName(b"S") => {
                        process_sreg_value(e, &mut curr_instruction_form, |sreg, val| sreg.s = val);
                    }
                    QName(b"V") => {
                        process_sreg_value(e, &mut curr_instruction_form, |sreg, val| sreg.v = val);
                    }
                    QName(b"Z") => {
                        process_sreg_value(e, &mut curr_instruction_form, |sreg, val| sreg.z = val);
                    }
                    QName(b"C") => {
                        process_sreg_value(e, &mut curr_instruction_form, |sreg, val| sreg.c = val);
                    }
                    QName(b"N") => {
                        process_sreg_value(e, &mut curr_instruction_form, |sreg, val| sreg.n = val);
                    }
                    // Clocks
                    QName(b"AVRe") => {
                        process_clock_value(e, &mut curr_instruction_form, |timing, val| {
                            timing.avre = val;
                        });
                    }
                    QName(b"AVRxm") => {
                        process_clock_value(e, &mut curr_instruction_form, |timing, val| {
                            timing.avrxm = val;
                        });
                    }
                    QName(b"AVRxt") => {
                        process_clock_value(e, &mut curr_instruction_form, |timing, val| {
                            timing.avrxt = val;
                        });
                    }
                    QName(b"AVRrc") => {
                        process_clock_value(e, &mut curr_instruction_form, |timing, val| {
                            timing.avrrc = val;
                        });
                    }
                    _ => {} // unknown event
                }
            }
            // end event
            Ok(Event::End(ref e)) => {
                match e.name() {
                    QName(b"Instruction") => {
                        // finish instruction
                        assert!(curr_instruction.arch != Arch::None);
                        instructions_map
                            .insert(curr_instruction.name.clone(), curr_instruction.clone());
                        curr_version = None;
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

    Ok(instructions_map.into_values().collect())
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
    let mut arch: Arch = Arch::None;

    loop {
        match reader.read_event() {
            // start event
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    QName(b"InstructionSet") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if b"name" == key.into_inner() {
                                arch = Arch::from_str(ustr::get_str(&value)).unwrap_or_else(|e| {
                                    panic!(
                                        "Unexpected Arch variant {} -- {e}",
                                        ustr::get_str(&value)
                                    )
                                });
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
                        assert!(curr_register.arch != Arch::None);
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

pub fn populate_name_to_register_map(
    arch: Arch,
    registers: &Vec<Register>,
    names_to_registers: &mut NameToRegisterMap,
) {
    for register in registers {
        for name in &register.get_associated_names() {
            names_to_registers.insert((arch, (*name).to_string()), register.clone());
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
pub fn populate_masm_nasm_fasm_mars_directives(xml_contents: &str) -> Result<Vec<Directive>> {
    let mut directives_map = HashMap::<String, Directive>::new();

    // iterate through the XML
    let mut reader = Reader::from_str(xml_contents);

    // ref to the assembler directive that's currently under construction
    let mut curr_directive = Directive::default();
    let mut in_desc = false;

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
                                    curr_directive.assembler = assembler;
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
        assert_ne!(directive.assembler, Assembler::None);
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
    let mut assembler = Assembler::None;

    loop {
        match reader.read_event() {
            // start event
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    QName(b"Assembler") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if b"name" == key.into_inner() {
                                assembler = Assembler::from_str(ustr::get_str(&value)).unwrap();
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

    // Since directive entries have their assembler labeled on a per-instance basis,
    // we check to make sure all of them have been assigned correctly
    for directive in directives_map.values() {
        assert_ne!(directive.assembler, Assembler::None);
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
pub fn populate_avr_directives(xml_contents: &str) -> Result<Vec<Directive>> {
    let mut directives_map = HashMap::<String, Directive>::new();

    // iterate through the XML
    let mut reader = Reader::from_str(xml_contents);

    // ref to the assembler directive that's currently under construction
    let mut curr_directive = Directive::default();
    let mut assembler = Assembler::None;

    loop {
        match reader.read_event() {
            // start event
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    QName(b"Assembler") => {
                        for attr in e.attributes() {
                            let Attribute { key, value } = attr.unwrap();
                            if b"name" == key.into_inner() {
                                assembler = Assembler::from_str(ustr::get_str(&value)).unwrap();
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
                                b"description" => {
                                    let description = ustr::get_str(&value);
                                    curr_directive.description =
                                        unescape(description).unwrap().to_string();
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

    // Since directive entries have their assembler labeled on a per-instance basis,
    // we check to make sure all of them have been assigned correctly
    for directive in directives_map.values() {
        assert_ne!(directive.assembler, Assembler::None);
    }

    Ok(directives_map.into_values().collect())
}

/// Parse the provided HTML contents and return a vector of all the directives based on that.
/// If parsing fails, the appropriate error will be returned instead.
///
/// <https://cc65.github.io/doc/ca65.html>
///
/// # Errors
///
/// This function is highly specialized to parse a single file and will panic or return
/// `Err` for most mal-formed/unexpected inputs
///
/// # Panics
///
/// This function is highly specialized to parse a single file and will panic or return
/// `Err` for most mal-formed/unexpected inputs
pub fn populate_ca65_directives(html_conts: &str) -> Result<Vec<Directive>> {
    let eat_lines = |lines: &mut Peekable<Lines<'_>>, empty: bool| {
        while let Some(line) = lines.peek() {
            if empty != line.is_empty() {
                break;
            }
            _ = lines.next().unwrap();
        }
    };
    let name_regex = Regex::new(r"<CODE>(?<name>.+)</CODE>").unwrap();
    let url_regex =
        Regex::new(r#"^<H2><A NAME=".+"></A> <A NAME="(?<fragment>[a-z, A-Z, 0-9, .]+)">"#)
            .unwrap();
    let mut directives = Vec::new();
    let start = {
        let start_marker = r##"<H2><A NAME="pseudo-variables"></A> <A NAME="s9">9.</A> <A HREF="#toc9">Pseudo variables</A></H2>"##;
        let section_start = html_conts.find(start_marker).unwrap();
        section_start + start_marker.len() + 1 // + 1 for '\n'
    };
    let mut lines = html_conts[start..].lines().peekable();
    eat_lines(&mut lines, true);
    _ = lines.next().unwrap(); // Extra info on pseudo variables
    _ = lines.next().unwrap();
    eat_lines(&mut lines, true);
    'outer: loop {
        // Consume lines until we find a section header
        loop {
            let Some(next) = lines.peek() else {
                break 'outer;
            };
            let next = next.trim();
            if next.starts_with("<H2><A NAME=\".") || next.starts_with("<H2><A NAME=\"*") {
                break;
            }
            _ = lines.next().unwrap();
        }

        let name_line = lines.next().unwrap();
        let name = {
            let Some(caps) = &name_regex.captures(name_line) else {
                // If this capture fails, we're at a section header rather than a subsection header
                // We don't care about section headers, since they don't document any information
                // about directives or anything else of importance. Just consume the lines and move on
                eat_lines(&mut lines, true);
                eat_lines(&mut lines, false);
                eat_lines(&mut lines, true);
                continue;
            };
            caps["name"].to_string()
        };
        let fragment = &url_regex.captures(name_line).unwrap()["fragment"];
        let url = format!("https://cc65.github.io/doc/ca65.html#{fragment}");
        assert_eq!(lines.next().unwrap().trim(), "</H2>");
        eat_lines(&mut lines, true);
        // get the description, remove anything inside carets
        let mut description = String::new();
        while !lines.peek().unwrap().is_empty() {
            let description_line = lines.next().unwrap();
            let len_before = description.len();
            let mut prev_idx = 0;
            for (i, c) in description_line.chars().enumerate() {
                match c {
                    '<' => {
                        #[allow(
                            clippy::sliced_string_as_bytes,
                            clippy::char_indices_as_byte_indices
                        )]
                        let bytes: Vec<u8> = description_line[prev_idx..i].as_bytes().to_vec();
                        let decoded = htmlentity::entity::decode(&bytes).to_string().unwrap();
                        description += &decoded;
                    }
                    '>' => prev_idx = i + 1,
                    _ => {}
                }
            }
            let line_len = description_line.len();
            // Not all lines end with a closing tag...
            if prev_idx < line_len - 1 {
                #[allow(clippy::sliced_string_as_bytes)]
                let bytes = description_line[prev_idx..description_line.len()].as_bytes();
                let decoded = htmlentity::entity::decode(bytes).to_string().unwrap();
                description += &decoded;
            }
            if description.len() != len_before {
                description.push(' ');
            }
        }
        let description = {
            while description.ends_with('\n') {
                _ = description.pop();
            }
            description.push('\n');
            description.trim().replace("  ", " ")
        };
        // Some entries cover two items, add both to the map
        if name.contains(',') {
            for alias in name.split(", ") {
                directives.push(Directive {
                    name: alias.trim().to_lowercase(),
                    signatures: Vec::new(),
                    description: description.clone(),
                    deprecated: false,
                    url: Some(url.clone()),
                    assembler: Assembler::Ca65,
                });
            }
        } else {
            directives.push(Directive {
                name: name.to_lowercase(),
                signatures: Vec::new(),
                description,
                deprecated: false,
                url: Some(url),
                assembler: Assembler::Ca65,
            });
        }
    }

    Ok(directives)
}

pub fn populate_name_to_directive_map(
    assem: Assembler,
    directives: &Vec<Directive>,
    names_to_directives: &mut NameToDirectiveMap,
) {
    for directive in directives {
        for name in &directive.get_associated_names() {
            names_to_directives.insert((assem, (*name).to_string()), directive.clone());
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
            eprintln!("Failed to resolve the cache file path - Error: {e}.");
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
                eprintln!("Failed to fetch documentation from {x86_online_docs} - Error: {e}.");
                return None;
            }
        }
    } else if let Some(ref path) = x86_cache_path {
        match get_x86_docs_cache(path) {
            Ok(docs) => docs,
            Err(e) => {
                eprintln!(
                    "Failed to fetch documentation from the cache: {} - Error: {e}.",
                    path.display()
                );
                return None;
            }
        }
    } else {
        eprintln!("Failed to fetch documentation from the cache - Invalid path.");
        return None;
    };

    // try to create the iterator to check if the data is valid
    // if the body produces an empty iterator, we attempt to clear the cache
    if body.split("<td>").skip(1).step_by(2).next().is_none() {
        eprintln!("Invalid docs contents.");
        if let Some(ref path) = x86_cache_path {
            eprintln!("Attempting to remove the cache file {}...", path.display());
            match std::fs::remove_file(path) {
                Ok(()) => {
                    eprintln!("Cache file removed.");
                }
                Err(e) => {
                    eprintln!("Failed to remove the cache file - Error: {e}.",);
                }
            }
        } else {
            eprintln!("Unable to clear the cache, invalid path.");
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
    println!("Fetching further documentation from the web -> {x86_online_docs}...");
    // grab the info from the web
    let contents = reqwest::blocking::get(x86_online_docs)?.text()?;
    Ok(contents)
}

fn get_x86_docs_cache(x86_cache_path: &PathBuf) -> Result<String, std::io::Error> {
    println!(
        "Fetching html page containing further documentation, from the cache -> {}...",
        x86_cache_path.display()
    );
    fs::read_to_string(x86_cache_path)
}

fn set_x86_docs_cache(contents: &str, x86_cache_path: &PathBuf) {
    println!("Writing to the cache file {}...", x86_cache_path.display());
    match fs::File::create(x86_cache_path) {
        Ok(mut cache_file) => {
            println!("Created the cache file {} .", x86_cache_path.display());
            match cache_file.write_all(contents.as_bytes()) {
                Ok(()) => {
                    println!("Populated the cache.");
                }
                Err(e) => {
                    eprintln!(
                        "Failed to write to the cache file {} - Error: {e}.",
                        x86_cache_path.display()
                    );
                }
            }
        }
        Err(e) => {
            eprintln!(
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
        let xml_conts_x86 = include_str!("../docs_store/opcodes/x86.xml");
        assert!(populate_instructions(xml_conts_x86).is_ok());

        if x86_cache_path.is_file() {
            std::fs::remove_file(&x86_cache_path).unwrap();
        }
        let xml_conts_x86_64 = include_str!("../docs_store/opcodes/x86_64.xml");
        assert!(populate_instructions(xml_conts_x86_64).is_ok());

        // Clean things up so we don't have an empty cache file
        if x86_cache_path.is_file() {
            std::fs::remove_file(&x86_cache_path).unwrap();
        }
    }
}
