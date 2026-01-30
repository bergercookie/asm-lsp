use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::{Display, Write as _},
    path::PathBuf,
    str::FromStr,
};

use bincode::{BorrowDecode, Encode};
use compile_commands::{CompilationDatabase, SourceFile};
use log::{info, warn};
use lsp_textdocument::TextDocuments;
use lsp_types::{CompletionItem, Uri};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, Display, EnumString};
use tree_sitter::{Parser, Tree};

use crate::{
    UriConversion, populate_name_to_directive_map, populate_name_to_instruction_map,
    populate_name_to_register_map, process_uri,
};

pub const BINCODE_CFG: bincode::config::Configuration = bincode::config::standard().with_no_limit();

// Instruction
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Encode, BorrowDecode)]
pub struct Instruction {
    pub name: String,
    pub summary: String,
    pub forms: Vec<InstructionForm>,
    pub asm_templates: Vec<String>,
    pub aliases: Vec<InstructionAlias>,
    pub url: Option<String>,
    pub arch: Arch,
}

impl Hoverable for Instruction {}
impl Completable for Instruction {}

impl Default for Instruction {
    fn default() -> Self {
        let name = String::new();
        let summary = String::new();
        let forms = vec![];
        let asm_templates = vec![];
        let aliases = vec![];
        let url = None;
        let arch = Arch::None;

        Self {
            name,
            summary,
            forms,
            asm_templates,
            aliases,
            url,
            arch,
        }
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // basic fields
        let header = format!("{} [{}]", &self.name, self.arch.as_ref());

        //let mut v: Vec<&str> = vec![&header, &self.summary, "\n", "## Forms", "\n"];
        let mut v: Vec<&str> = vec![&header, &self.summary, "\n"];

        if !self.forms.is_empty() {
            v.append(&mut vec!["## Forms", "\n"]);
        }

        // instruction forms
        let instruction_form_strs: Vec<String> =
            self.forms.iter().map(|f| format!("{f}")).collect();
        for item in &instruction_form_strs {
            v.push(item.as_str());
        }

        if !self.asm_templates.is_empty() {
            v.append(&mut vec!["## Templates", "\n"]);
        }
        // instruction templates
        let instruction_template_strs: Vec<String> = self
            .asm_templates
            .iter()
            .map(|f| format!(" + `{}`", f.as_str()))
            .collect();
        for item in &instruction_template_strs {
            v.push(item.as_str());
        }

        if !self.aliases.is_empty() {
            v.append(&mut vec!["## Aliases", "\n"]);
        }

        // instruction aliases
        let instruction_alias_strs: Vec<String> =
            self.aliases.iter().map(|f| format!("{f}\n")).collect();
        for item in &instruction_alias_strs {
            v.push(item.as_str());
        }

        // url
        let more_info: String;
        if let Some(url) = &self.url {
            more_info = format!("\nMore info: {url}");
            v.push(&more_info);
        }

        let s = v.join("\n");
        write!(f, "{s}")?;
        Ok(())
    }
}

impl<'own> Instruction {
    /// Add a new form at the current instruction
    pub fn push_form(&mut self, form: InstructionForm) {
        self.forms.push(form);
    }

    /// Add a new alias at the current instruction
    pub fn push_alias(&mut self, form: InstructionAlias) {
        self.aliases.push(form);
    }

    /// Get the primary names
    #[must_use]
    pub fn get_primary_names(&'own self) -> Vec<&'own str> {
        let names = vec![self.name.as_ref()];

        names
    }

    /// Get the names of all the associated commands (includes Go and Gas forms)
    #[must_use]
    pub fn get_associated_names(&'own self) -> Vec<&'own str> {
        let mut names = Vec::<&'own str>::new();

        for f in &self.forms {
            for name in [&f.gas_name, &f.go_name, &f.z80_name]
                .iter()
                .copied()
                .flatten()
            {
                names.push(name);
            }
        }

        names
    }
}

// TODO: Rework this to use a tagged union...
// InstructionForm
#[derive(
    Default, Eq, PartialEq, Hash, Debug, Clone, Serialize, Deserialize, Encode, BorrowDecode,
)]
pub struct InstructionForm {
    // --- Gas/Go-Specific Information ---
    pub gas_name: Option<String>,
    pub go_name: Option<String>,
    pub mmx_mode: Option<MMXMode>,
    pub xmm_mode: Option<XMMMode>,
    pub cancelling_inputs: Option<bool>,
    pub nacl_version: Option<u8>,
    pub nacl_zero_extends_outputs: Option<bool>,
    // --- Z80-Specific Information ---
    pub z80_name: Option<String>,
    pub z80_form: Option<String>,
    pub z80_opcode: Option<String>,
    pub z80_timing: Option<Z80Timing>,
    // --- Avr-Specific Information ---
    pub avr_mneumonic: Option<String>,
    pub avr_summary: Option<String>,
    pub avr_version: Option<String>,
    pub avr_timing: Option<AvrTiming>,
    pub avr_status_register: Option<AvrStatusRegister>,
    // --- Assembler/Architecture Agnostic Info ---
    pub isa: Option<ISA>,
    pub operands: Vec<Operand>,
    pub urls: Vec<String>,
}

impl std::fmt::Display for InstructionForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        if let Some(val) = &self.gas_name {
            write!(s, "*GAS*: {val} | ")?;
        }
        if let Some(val) = &self.go_name {
            write!(s, "*GO*: {val} | ")?;
        }
        if let Some(val) = &self.z80_form {
            write!(s, "*Z80*: {val} | ")?;
        }
        if let Some(val) = &self.avr_mneumonic {
            let version_str = self
                .avr_version
                .as_ref()
                .map_or_else(String::new, |version| format!(" ({version})"));
            write!(s, "*AVR*: {val}{version_str} | ")?;
        }

        if let Some(val) = &self.mmx_mode {
            write!(s, "*MMX*: {} | ", val.as_ref())?;
        }
        if let Some(val) = &self.xmm_mode {
            write!(s, "*XMM*: {} | ", val.as_ref())?;
        }
        if let Some(val) = &self.z80_opcode {
            if val.contains(',') {
                write!(s, "*Opcodes*: {val} | ")?;
            } else {
                write!(s, "*Opcode*: {val} | ")?;
            }
        }

        // cancelling inputs
        // nacl_version
        // nacl_zero_extends_outputs

        // ISA
        if let Some(val) = &self.isa {
            write!(s, "*ISA*: {} | ", val.as_ref())?;
        }

        if !s.is_empty() {
            s = format!("- {}\n\n", &s[..s.len() - 3]);
        }

        // Operands
        let operands_str: String = if self.operands.is_empty() {
            String::new()
        } else {
            self.operands
                .iter()
                .map(|op| {
                    let mut s = format!("  + {:<8}", format!("[{}]", op.type_.as_ref()));
                    if let Some(input) = op.input {
                        write!(s, " input = {input:<5} ").unwrap();
                    }
                    if let Some(output) = op.output {
                        write!(s, " output = {output:<5}").unwrap();
                    }
                    if let Some(extended_size) = op.extended_size {
                        write!(s, " extended-size = {extended_size}").unwrap();
                    }

                    s.trim_end().to_owned()
                })
                .collect::<Vec<String>>()
                .join("\n")
        };

        s += &operands_str;

        if let Some(ref timing) = self.z80_timing {
            write!(s, "\n  + {timing}")?;
        }

        if let Some(summary) = &self.avr_summary {
            write!(s, "\n\n{summary}")?;
        }
        if let Some(sreg) = &self.avr_status_register {
            write!(s, "\n\n{sreg}")?;
        }
        if let Some(ref timing) = self.avr_timing {
            writeln!(s, "\n\n{timing}")?;
        }

        for url in &self.urls {
            writeln!(s, "\n  + More info: {url}")?;
        }

        write!(f, "{s}")?;
        Ok(())
    }
}

// InstructionAlias
#[derive(
    Default, Eq, PartialEq, Hash, Debug, Clone, Serialize, Deserialize, Encode, BorrowDecode,
)]
pub struct InstructionAlias {
    pub title: String,
    pub summary: String,
    pub asm_templates: Vec<String>,
}

impl std::fmt::Display for InstructionAlias {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = self.title.clone();

        if !self.summary.is_empty() {
            write!(s, "\n {}", self.summary)?;
        }

        for template in &self.asm_templates {
            write!(s, "\n + `{template}`")?;
        }

        write!(f, "{s}")?;
        Ok(())
    }
}

#[derive(
    Debug, Default, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize, Encode, BorrowDecode,
)]
pub enum Z80TimingValue {
    #[default]
    Unknown,
    Val(u8),
}

impl Display for Z80TimingValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => {
                write!(f, "?")?;
            }
            Self::Val(val) => {
                write!(f, "{val}")?;
            }
        }

        Ok(())
    }
}

impl FromStr for Z80TimingValue {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq("?") {
            Ok(Self::Unknown)
        } else {
            match s.parse::<u8>() {
                Ok(val) => Ok(Self::Val(val)),
                Err(e) => Err(e),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, Serialize, Deserialize, Encode, BorrowDecode)]
pub enum Z80TimingInfo {
    OneNum(Z80TimingValue), // better names here?
    TwoNum((Z80TimingValue, Z80TimingValue)),
    ThreeNum((Z80TimingValue, Z80TimingValue, Z80TimingValue)),
}

impl Default for Z80TimingInfo {
    fn default() -> Self {
        Self::OneNum(Z80TimingValue::Unknown)
    }
}

impl Display for Z80TimingInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OneNum(num) => {
                write!(f, "{num}")?;
            }
            Self::TwoNum((num1, num2)) => {
                write!(f, "{num1}/{num2}")?;
            }
            Self::ThreeNum((num1, num2, num3)) => {
                write!(f, "{num1}/{num2}/{num3}")?;
            }
        }
        Ok(())
    }
}

impl FromStr for Z80TimingInfo {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('/') || s.ends_with('/') {
            return Err(String::from("Cannot have an empty num value"));
        }

        let pieces: Vec<Result<Z80TimingValue, std::num::ParseIntError>> =
            s.split('/').map(str::parse).collect();

        match pieces.len() {
            1 => pieces[0].as_ref().map_or_else(
                |_| Err(String::from("Failed to parse one timing value")),
                |num| Ok(Self::OneNum(*num)),
            ),
            2 => match (&pieces[0], &pieces[1]) {
                (Ok(num1), Ok(num2)) => Ok(Self::TwoNum((*num1, *num2))),
                _ => Err(String::from("Failed to parse one or more timing values")),
            },
            3 => match (&pieces[0], &pieces[1], &pieces[2]) {
                (Ok(num1), Ok(num2), Ok(num3)) => Ok(Self::ThreeNum((*num1, *num2, *num3))),
                _ => Err(String::from("Failed to parse one or more timing values")),
            },
            n => Err(format!("Expected 1-3 timing values, got {n}")),
        }
    }
}

#[derive(
    Default, Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, Encode, BorrowDecode,
)]
pub struct Z80Timing {
    pub z80: Z80TimingInfo,
    pub z80_plus_m1: Z80TimingInfo,
    pub r800: Z80TimingInfo,
    pub r800_plus_wait: Z80TimingInfo,
}

impl Display for Z80Timing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Z80: {}, Z80 + M1: {}, R800: {}, R800 + Wait: {}",
            self.z80, self.z80_plus_m1, self.r800, self.z80_plus_m1
        )?;
        Ok(())
    }
}

#[derive(
    Default, Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, Encode, BorrowDecode,
)]
pub struct AvrTiming {
    pub avre: Option<String>,
    pub avrxm: Option<String>,
    pub avrxt: Option<String>,
    pub avrrc: Option<String>,
}

impl Display for AvrTiming {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Timing: ")?;
        let mut has_prev = if let Some(ref cycles) = self.avre {
            write!(f, "AVRE: {cycles}")?;
            true
        } else {
            false
        };
        if let Some(ref cycles) = self.avrxm {
            if has_prev {
                write!(f, " | ")?;
            }
            write!(f, "AVRXM: {cycles}")?;
            has_prev = true;
        }
        if let Some(ref cycles) = self.avrxt {
            if has_prev {
                write!(f, " | ")?;
            }
            write!(f, "AVRXT: {cycles}")?;
            has_prev = true;
        }
        if let Some(ref cycles) = self.avrrc {
            if has_prev {
                write!(f, " | ")?;
            }
            write!(f, "AVRRC: {cycles}")?;
        }

        Ok(())
    }
}

#[derive(
    Default, Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, Encode, BorrowDecode,
)]
pub struct AvrStatusRegister {
    pub i: char,
    pub t: char,
    pub h: char,
    pub s: char,
    pub v: char,
    pub n: char,
    pub c: char,
    pub z: char,
}

impl std::fmt::Display for AvrStatusRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "I T H S V N Z C")?;
        writeln!(
            f,
            "{} {} {} {} {} {} {} {}",
            self.i, self.t, self.h, self.s, self.v, self.n, self.c, self.z
        )?;
        Ok(())
    }
}

// Directive
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, BorrowDecode)]
pub struct Directive {
    pub name: String,
    pub signatures: Vec<String>,
    pub description: String,
    pub deprecated: bool,
    pub url: Option<String>,
    pub assembler: Assembler,
}

impl Hoverable for Directive {}
impl Completable for Directive {}

impl Default for Directive {
    fn default() -> Self {
        let name = String::new();
        let signatures = vec![];
        let description = String::new();
        let deprecated = false;
        let url = None;
        let assembler = Assembler::None;

        Self {
            name,
            signatures,
            description,
            deprecated,
            url,
            assembler,
        }
    }
}

impl std::fmt::Display for Directive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // basic fields
        // &self.assembler {
        let header = format!(
            "{} [{}]{}",
            &self.name,
            self.assembler,
            if self.deprecated {
                "\n**DEPRECATED**"
            } else {
                ""
            }
        );

        let mut v: Vec<&str> = vec![&header, &self.description, "\n"];

        // signature(s)
        let mut sigs = String::new();
        for sig in &self.signatures {
            writeln!(sigs, "- {sig}")?;
        }
        v.push(&sigs);

        // url
        let more_info: String;
        match &self.url {
            None => {}
            Some(url_) => {
                more_info = format!("\nMore info: {url_}");
                v.push(&more_info);
            }
        }

        let s = v.join("\n");
        write!(f, "{}", s.trim())?;
        Ok(())
    }
}

#[derive(Default, Debug, Clone)]
pub enum Z80Register8Bit {
    #[default]
    A,
    AShadow,
    F,
    FShadow,
    B,
    BShadow,
    C,
    CShadow,
    D,
    DShadow,
    E,
    EShadow,
    H,
    HShadow,
    L,
    LShadow,
}

#[derive(Default, Debug, Clone)]
pub enum Z80Register16Bit {
    AF,
    AFShadow,
    BC,
    BCShadow,
    DE,
    DEShadow,
    #[default]
    HL,
    HLShadow,
}

impl<'own> Directive {
    /// get the names of all the associated directives
    #[must_use]
    pub fn get_associated_names(&'own self) -> Vec<&'own str> {
        let names = vec![self.name.as_ref()];

        names
    }
}

// Register
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, BorrowDecode)]
pub struct Register {
    pub name: String,
    pub description: Option<String>,
    pub reg_type: Option<RegisterType>,
    pub width: Option<RegisterWidth>,
    pub flag_info: Vec<RegisterBitInfo>,
    pub arch: Arch,
    pub url: Option<String>,
}

impl Hoverable for Register {}
impl Completable for Register {}

impl Default for Register {
    fn default() -> Self {
        let name = String::new();
        let description = None;
        let reg_type = None;
        let width = None;
        let flag_info = vec![];
        let arch = Arch::None;
        let url = None;

        Self {
            name,
            description,
            reg_type,
            width,
            flag_info,
            arch,
            url,
        }
    }
}

impl std::fmt::Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // basic fields
        let header = format!("{} [{}]", &self.name.to_uppercase(), self.arch.as_ref());

        let mut v: Vec<String> = if let Some(description_) = &self.description {
            vec![header, description_.clone(), String::from("\n")]
        } else {
            vec![header, String::from("\n")]
        };

        // Register Type
        if let Some(reg_type_) = &self.reg_type {
            let reg_type = format!("Type: {reg_type_}");
            v.push(reg_type);
        }

        // Register Width
        if let Some(reg_width_) = self.width {
            let reg_width = format!("Width: {reg_width_}");
            v.push(reg_width);
        }

        // Bit-mask flag meanings if applicable
        if !self.flag_info.is_empty() {
            let flag_heading = String::from("\n## Flags:");
            v.push(flag_heading);

            let flags: Vec<String> = self
                .flag_info
                .iter()
                .map(|flag| format!("{flag}"))
                .collect();
            for flag in &flags {
                v.push(flag.clone());
            }
        }

        // TODO: URL support
        if let Some(url_) = &self.url {
            let more_info = format!("\nMore info: {url_}");
            v.push(more_info);
        }

        let s = v.join("\n");
        write!(f, "{s}")?;
        Ok(())
    }
}

impl<'own> Register {
    /// Add a new bit flag entry at the current instruction
    pub fn push_flag(&mut self, flag: RegisterBitInfo) {
        self.flag_info.push(flag);
    }

    /// get the names of all the associated registers
    #[must_use]
    pub fn get_associated_names(&'own self) -> Vec<&'own str> {
        let names = vec![self.name.as_ref()];

        names
    }
}

// helper structs, types and functions
#[derive(Debug, Clone, Default)]
pub struct NameToInfoMaps {
    pub instructions: NameToInstructionMap,
    pub registers: NameToRegisterMap,
    pub directives: NameToDirectiveMap,
}

pub type NameToInstructionMap = HashMap<(Arch, String), Instruction>;

pub type NameToRegisterMap = HashMap<(Arch, String), Register>;

pub type NameToDirectiveMap = HashMap<(Assembler, String), Directive>;

pub trait Hoverable: Display + Clone {}
pub trait Completable: Display {}
pub trait ArchOrAssembler: Clone + Copy {}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    AsRefStr,
    Serialize,
    Deserialize,
    Encode,
    BorrowDecode,
)]
pub enum XMMMode {
    SSE,
    AVX,
}

#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    Hash,
    EnumString,
    AsRefStr,
    Serialize,
    Deserialize,
    Encode,
    BorrowDecode,
)]
pub enum MMXMode {
    FPU,
    MMX,
}

#[allow(non_camel_case_types)]
#[derive(
    Debug,
    Hash,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
    AsRefStr,
    Serialize,
    Deserialize,
    Encode,
    BorrowDecode,
    JsonSchema,
)]
pub enum Arch {
    #[strum(serialize = "x86")]
    #[serde(rename = "x86")]
    X86,
    #[strum(serialize = "x86-64")]
    #[serde(rename = "x86-64")]
    X86_64,
    /// enables both `Arch::X86` and `Arch::X86_64`
    #[strum(serialize = "x86/x86-64")]
    #[serde(rename = "x86/x86-64")]
    X86_AND_X86_64,
    #[strum(serialize = "arm")]
    #[serde(rename = "arm")]
    ARM,
    #[strum(serialize = "arm64")]
    #[serde(rename = "arm64")]
    ARM64,
    #[strum(serialize = "riscv")]
    #[serde(rename = "riscv")]
    RISCV,
    #[strum(serialize = "z80")]
    #[serde(rename = "z80")]
    Z80,
    #[strum(serialize = "6502")]
    #[serde(rename = "6502")]
    MOS6502,
    #[strum(serialize = "power-isa")]
    #[serde(rename = "power-isa")]
    PowerISA,
    #[strum(serialize = "AVR")]
    #[serde(rename = "AVR")] // TODO: lower-case this in the generation code
    Avr,
    #[strum(serialize = "mips")]
    #[serde(rename = "mips")]
    Mips,
    /// For testing purposes *only*. This is not a valid config option
    #[serde(skip)]
    None,
}

impl ArchOrAssembler for Arch {}

impl Arch {
    /// Setup registers for a particular architecture
    ///
    /// # Panics
    ///
    /// Panics if unable to deserialize `Register`
    pub fn setup_registers(self, names_to_registers: &mut HashMap<(Self, String), Register>) {
        macro_rules! load_registers_with_path {
            ($arch:expr, $path:literal) => {{
                let start = std::time::Instant::now();
                let serialized_regs = include_bytes!($path);
                let registers = bincode::borrow_decode_from_slice::<Vec<Register>, _>(serialized_regs, BINCODE_CFG)
                    .unwrap_or_else(|e| panic!("Error deserializing {} registers -- {}\nRegenerate serialized data with regenerate.sh", $arch, e)).0;

                info!(
                    "{} register set loaded in {}ms",
                    $arch,
                    start.elapsed().as_millis()
                );

                populate_name_to_register_map($arch, &registers, names_to_registers);
            }};
        }

        match self {
            Self::ARM => load_registers_with_path!(Self::ARM, "serialized/registers/arm"),
            Self::ARM64 => load_registers_with_path!(Self::ARM64, "serialized/registers/arm"),
            Self::Avr => load_registers_with_path!(Self::Avr, "serialized/registers/avr"),
            Self::Mips => load_registers_with_path!(Self::Mips, "serialized/registers/mips"),
            Self::MOS6502 => load_registers_with_path!(Self::MOS6502, "serialized/registers/6502"),
            Self::PowerISA => {
                load_registers_with_path!(Self::PowerISA, "serialized/registers/power-isa");
            }
            Self::RISCV => load_registers_with_path!(Self::RISCV, "serialized/registers/riscv"),
            Self::X86 => load_registers_with_path!(Self::X86, "serialized/registers/x86"),
            Self::X86_64 => load_registers_with_path!(Self::X86_64, "serialized/registers/x86_64"),
            Self::X86_AND_X86_64 => {
                load_registers_with_path!(Self::X86, "serialized/registers/x86");
                load_registers_with_path!(Self::X86_64, "serialized/registers/x86_64");
            }
            Self::Z80 => load_registers_with_path!(Self::Z80, "serialized/registers/z80"),
            Self::None => unreachable!(),
        }
    }

    /// Setup instructions for a particular architecture
    ///
    /// # Panics
    ///
    /// Panics if unable to deserialize `Instruction`s
    pub fn setup_instructions(
        self,
        assembler: Option<Assembler>,
        names_to_instructions: &mut HashMap<(Self, String), Instruction>,
    ) {
        macro_rules! load_instructions_with_path {
            ($arch:expr, $path:literal) => {{
                let start = std::time::Instant::now();
                let serialized_instrs = include_bytes!($path);
                let instructions = bincode::borrow_decode_from_slice::<Vec<Instruction>, _>(serialized_instrs, BINCODE_CFG)
                    .unwrap_or_else(|e| panic!("Error deserializing {} instructions -- {}\nRegenerate serialized data with regenerate.sh", $arch, e))
                    .0;

                info!(
                    "{} instruction set loaded in {}ms",
                    $arch,
                    start.elapsed().as_millis()
                );

                populate_name_to_instruction_map($arch, &instructions, names_to_instructions);
            }};
        }

        match self {
            Self::ARM => load_instructions_with_path!(Self::ARM, "serialized/opcodes/arm"),
            // NOTE: Actually, the arm file are all arm64 so we needed to get
            // the arm32 versions then do the below
            Self::ARM64 => load_instructions_with_path!(Self::ARM64, "serialized/opcodes/arm"),
            Self::Avr => load_instructions_with_path!(Self::Avr, "serialized/opcodes/avr"),
            Self::MOS6502 => load_instructions_with_path!(Self::MOS6502, "serialized/opcodes/6502"),
            Self::Mips => {
                load_instructions_with_path!(Self::Mips, "serialized/opcodes/mips");
                if Some(Assembler::Mars) == assembler {
                    load_instructions_with_path!(Self::Mips, "serialized/opcodes/mars");
                }
            }
            Self::PowerISA => {
                load_instructions_with_path!(Self::PowerISA, "serialized/opcodes/power-isa");
            }
            Self::RISCV => load_instructions_with_path!(Self::RISCV, "serialized/opcodes/riscv"),
            Self::X86 => load_instructions_with_path!(Self::X86, "serialized/opcodes/x86"),
            Self::X86_64 => load_instructions_with_path!(Self::X86_64, "serialized/opcodes/x86_64"),
            Self::X86_AND_X86_64 => {
                load_instructions_with_path!(Self::X86, "serialized/opcodes/x86");
                load_instructions_with_path!(Self::X86_64, "serialized/opcodes/x86_64");
            }
            Self::Z80 => load_instructions_with_path!(Self::Z80, "serialized/opcodes/z80"),
            Self::None => unreachable!(),
        }
    }
}

impl Default for Arch {
    fn default() -> Self {
        match () {
            () if cfg!(target_arch = "x86") => {
                info!("Detected host arch as x86");
                Self::X86
            }
            () if cfg!(target_arch = "arm") => {
                info!("Detected host arch as arm");
                Self::ARM
            }
            () if cfg!(target_arch = "aarch64") => {
                info!("Detected host arch as aarch64");
                Self::ARM64
            }
            () if cfg!(target_arch = "riscv32") => {
                info!("Detected host arch as riscv32");
                Self::RISCV
            }
            () if cfg!(target_arch = "riscv64") => {
                info!("Detected host arch as riscv64");
                Self::RISCV
            }
            () => {
                if cfg!(target_arch = "x86_64") {
                    info!("Detected host arch as x86-64");
                } else {
                    info!("Failed to detect host arch, defaulting to x86-64");
                }
                Self::X86_64 // Somewhat arbitrary default
            }
        }
    }
}

impl std::fmt::Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ARM => write!(f, "arm")?,
            Self::ARM64 => write!(f, "arm64")?,
            Self::Avr => write!(f, "avr")?,
            Self::Mips => write!(f, "mips")?,
            Self::MOS6502 => write!(f, "6502")?,
            Self::PowerISA => write!(f, "power-isa")?,
            Self::RISCV => write!(f, "riscv")?,
            Self::X86 => write!(f, "x86")?,
            Self::X86_64 => write!(f, "x86-64")?,
            Self::X86_AND_X86_64 => write!(f, "x86/x86-64")?,
            Self::Z80 => write!(f, "z80")?,
            Self::None => write!(f, "None")?,
        }
        Ok(())
    }
}

#[derive(
    Debug,
    Display,
    Default,
    Hash,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
    AsRefStr,
    Serialize,
    Deserialize,
    Encode,
    BorrowDecode,
    JsonSchema,
)]
pub enum Assembler {
    #[strum(serialize = "avr")]
    #[serde(rename = "avr")]
    Avr,
    #[strum(serialize = "ca65")]
    #[serde(rename = "ca65")]
    Ca65,
    #[strum(serialize = "fasm")]
    #[serde(rename = "fasm")]
    Fasm,
    #[default]
    #[strum(serialize = "gas")]
    #[serde(rename = "gas")]
    Gas,
    #[strum(serialize = "go")]
    #[serde(rename = "go")]
    Go,
    #[strum(serialize = "mars")]
    #[serde(rename = "mars")]
    Mars,
    #[strum(serialize = "masm")]
    #[serde(rename = "masm")]
    Masm,
    #[strum(serialize = "nasm")]
    #[serde(rename = "nasm")]
    Nasm,
    #[serde(skip)]
    None,
}

impl ArchOrAssembler for Assembler {}

impl Assembler {
    /// Setup directives for an assembler
    ///
    /// # Panics
    ///
    /// Panics if unable to deserialize `Directive`s
    pub fn setup_directives(self, names_to_directives: &mut HashMap<(Self, String), Directive>) {
        macro_rules! load_directives_with_path {
            ($assembler:expr, $path:literal) => {{
                let start = std::time::Instant::now();
                let serialized_dirs = include_bytes!($path);
                let directives = bincode::borrow_decode_from_slice::<Vec<Directive>, _>(serialized_dirs, BINCODE_CFG)
                    .unwrap_or_else(|e| panic!("Error deserializing {} directives -- {}\nRegenerate serialized data with regenerate.sh", $assembler, e))
                    .0;

                info!(
                    "{} directive set loaded in {}ms",
                    $assembler,
                    start.elapsed().as_millis()
                );

                populate_name_to_directive_map($assembler, &directives, names_to_directives);
            }};
        }

        match self {
            Self::Avr => load_directives_with_path!(Self::Avr, "serialized/directives/avr"),
            Self::Ca65 => load_directives_with_path!(Self::Ca65, "serialized/directives/ca65"),
            Self::Fasm => load_directives_with_path!(Self::Fasm, "serialized/directives/fasm"),
            Self::Gas => load_directives_with_path!(Self::Gas, "serialized/directives/gas"),
            Self::Go => warn!("There is currently no Go-specific assembler documentation"),
            Self::Mars => load_directives_with_path!(Self::Mars, "serialized/directives/mars"),
            Self::Masm => load_directives_with_path!(Self::Masm, "serialized/directives/masm"),
            Self::Nasm => load_directives_with_path!(Self::Nasm, "serialized/directives/nasm"),
            Self::None => unreachable!(),
        }
    }
}

#[derive(
    Debug,
    Hash,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
    AsRefStr,
    Display,
    Serialize,
    Deserialize,
    Encode,
    BorrowDecode,
)]
pub enum RegisterType {
    #[strum(serialize = "General Purpose Register")]
    GeneralPurpose,
    #[strum(serialize = "Special Purpose Register")]
    SpecialPurpose,
    #[strum(serialize = "Pointer Register")]
    Pointer,
    #[strum(serialize = "Segment Register")]
    Segment,
    #[strum(serialize = "Flag Register")]
    Flag,
    #[strum(serialize = "Control Register")]
    Control,
    #[strum(serialize = "Extended Control Register")]
    ExtendedControl,
    #[strum(serialize = "Machine State Register")]
    MSR,
    #[strum(serialize = "Debug Register")]
    Debug,
    #[strum(serialize = "Test Register")]
    Test,
    #[strum(serialize = "Protected Mode Register")]
    ProtectedMode,
    #[strum(serialize = "Floating Point Register")]
    FloatingPoint,
}

#[derive(
    Debug,
    Hash,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
    AsRefStr,
    Display,
    Serialize,
    Deserialize,
    Encode,
    BorrowDecode,
)]
pub enum RegisterWidth {
    #[strum(serialize = "512 bits")]
    Bits512,
    #[strum(serialize = "256 bits")]
    Bits256,
    #[strum(serialize = "128 bits")]
    Bits128,
    #[strum(serialize = "32(64) bits")]
    Bits32Or64,
    #[strum(serialize = "64 bits")]
    Bits64,
    #[strum(serialize = "48 bits")]
    Bits48,
    #[strum(serialize = "32 bits")]
    Bits32,
    #[strum(serialize = "16 bits")]
    Bits16,
    #[strum(serialize = "8 bits")]
    Bits8,
    #[strum(serialize = "8 high bits of lower 16 bits")]
    Upper8Lower16,
    #[strum(serialize = "8 lower bits")]
    Lower8Lower16,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Default, Deserialize, Encode, BorrowDecode,
)]
pub struct RegisterBitInfo {
    pub bit: u32,
    pub label: String,
    pub description: String,
    pub pae: String,
    pub long_mode: String,
}

impl std::fmt::Display for RegisterBitInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = if self.label.is_empty() {
            format!("{:2}: {}", self.bit, self.description)
        } else {
            format!("{:2}: {} - {}", self.bit, self.label, self.description)
        };
        if !self.pae.is_empty() {
            write!(s, ", PAE: {}", self.pae)?;
        }
        if !self.long_mode.is_empty() {
            write!(s, " , Long Mode: {}", self.long_mode)?;
        }

        write!(f, "{s}")?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RootConfig {
    pub default_config: Option<Config>,
    #[serde(rename = "project")]
    pub projects: Option<Vec<ProjectConfig>>,
}

impl Default for RootConfig {
    fn default() -> Self {
        Self {
            default_config: Some(Config::default()),
            projects: None,
        }
    }
}

impl RootConfig {
    /// Returns the `Project` associated with `uri`
    ///
    /// # Panics
    ///
    /// Will panic if `req_uri` cannot be canonicalized
    #[must_use]
    pub fn get_project<'a>(&'a self, request_path: &PathBuf) -> Option<&'a ProjectConfig> {
        if let Some(projects) = &self.projects {
            for project in projects {
                if (project.path.is_dir() && request_path.starts_with(&project.path))
                    || (project.path.is_file() && request_path.eq(&project.path))
                {
                    return Some(project);
                }
            }
        }

        None
    }

    /// Returns the project-specific `Config` associated with `uri`, or the default if no
    /// matching configuration is found
    ///
    /// # Panics
    ///
    /// Will panic if `self` does not have a valid configuration for `req_uri`, meaning
    /// no default config and no applicable project config
    pub fn get_config<'a>(&'a self, req_uri: &'a Uri) -> &'a Config {
        let request_path = match process_uri(req_uri) {
            UriConversion::Canonicalized(p) => p,
            UriConversion::Unchecked(p) => {
                warn!(
                    "Failed to canonicalize request path {}, using {}",
                    req_uri.path().as_str(),
                    p.display()
                );
                p
            }
        };
        if let Some(project) = self.get_project(&request_path) {
            info!(
                "Selected project config with path \"{}\"",
                project.path.display()
            );
            return &project.config;
        }
        if let Some(root) = &self.default_config {
            info!("Selected root config");
            return root;
        }

        panic!(
            "Invalid configuration for \"{}\" -- Must contain a per-project configuration or default",
            req_uri.path()
        );
    }

    /// # Panics
    ///
    /// Will panic if `self.default_config` is `None`
    #[must_use]
    pub fn effective_arches(&self) -> Vec<Arch> {
        let mut arch_set = HashSet::new();

        // NOTE: `self.default_config` is assumed to be set to `Some` in
        // `get_root_config`
        assert!(self.default_config.is_some());
        arch_set.insert(self.default_config.as_ref().unwrap().instruction_set);
        if let Some(ref projects) = self.projects {
            for project in projects {
                arch_set.insert(project.config.instruction_set);
            }
        }

        arch_set.into_iter().collect()
    }

    /// # Panics
    ///
    /// Will panic if `self.default_config` is `None`
    #[must_use]
    pub fn effective_assemblers(&self) -> Vec<Assembler> {
        let mut assembler_set = HashSet::new();

        // NOTE: `self.default_config` is assumed to be set to `Some` in
        // `get_root_config`
        assert!(self.default_config.is_some());
        assembler_set.insert(self.default_config.as_ref().unwrap().assembler);
        if let Some(ref projects) = self.projects {
            for project in projects {
                assembler_set.insert(project.config.assembler);
            }
        }

        assembler_set.into_iter().collect()
    }

    #[must_use]
    pub fn is_isa_enabled(&self, isa: Arch) -> bool {
        if let Some(ref root) = self.default_config
            && root.is_isa_enabled(isa)
        {
            return true;
        }

        if let Some(ref projects) = self.projects {
            for project in projects {
                if project.config.is_isa_enabled(isa) {
                    return true;
                }
            }
        }

        false
    }

    #[must_use]
    pub fn is_assembler_enabled(&self, assembler: Assembler) -> bool {
        if let Some(ref root) = self.default_config
            && root.is_assembler_enabled(assembler)
        {
            return true;
        }

        if let Some(ref projects) = self.projects {
            for project in projects {
                if project.config.is_assembler_enabled(assembler) {
                    return true;
                }
            }
        }

        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectConfig {
    // Path to a directory or source file on which this config applies
    // Can be relative to the server's root directory, or absolute
    pub path: PathBuf,
    /// Config for this project. If `path` is a directory, applies to all files
    /// and subdirectories. If `path` is a file, just applies to that file
    #[serde(flatten)]
    pub config: Config,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Config {
    pub version: Option<String>,
    pub assembler: Assembler,
    pub instruction_set: Arch,
    pub opts: Option<ConfigOptions>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: None,
            assembler: Assembler::default(),
            instruction_set: Arch::default(),
            opts: Some(ConfigOptions::default()),
        }
    }
}

impl Config {
    #[allow(clippy::missing_const_for_fn)]
    #[must_use]
    pub fn get_compiler(&self) -> Option<&str> {
        match self.opts {
            Some(ConfigOptions {
                compiler: Some(ref compiler),
                ..
            }) => Some(compiler),
            _ => None,
        }
    }

    #[must_use]
    pub const fn get_compile_flags_txt(&self) -> Option<&Vec<String>> {
        match self.opts {
            Some(ConfigOptions {
                compile_flags_txt: Some(ref flags),
                ..
            }) => Some(flags),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_isa_enabled(&self, isa: Arch) -> bool {
        match self.instruction_set {
            Arch::X86_AND_X86_64 => {
                isa == Arch::X86 || isa == Arch::X86_64 || isa == Arch::X86_AND_X86_64
            }
            // TODO: Same treatment as above for ARM32/ARM64
            arch => isa == arch,
        }
    }

    #[must_use]
    pub fn is_assembler_enabled(&self, assembler: Assembler) -> bool {
        self.assembler == assembler
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ConfigOptions {
    // Specify compiler to generate diagnostics via `compile_flags.txt`
    pub compiler: Option<String>,
    // List of compile flags to override compilation behavior for this config.
    // Do not include the input source file as an argument
    // *Not* a path to `compile_flags.txt`
    pub compile_flags_txt: Option<Vec<String>>,
    // Turn diagnostics feature on/off
    pub diagnostics: Option<bool>,
    // Turn default diagnostics (no compilation db detected) on/off
    pub default_diagnostics: Option<bool>,
}

impl Default for ConfigOptions {
    fn default() -> Self {
        Self {
            compiler: None,
            compile_flags_txt: None,
            diagnostics: Some(true),
            default_diagnostics: Some(true),
        }
    }
}

// Instruction Set Architecture
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    AsRefStr,
    Serialize,
    Deserialize,
    Encode,
    BorrowDecode,
)]
pub enum ISA {
    #[strum(serialize = "RAO-INT")]
    RAOINT,
    GFNI,
    VAES,
    VPCLMULQDQ,
    RDTSC,
    RDTSCP,
    CPUID,
    CMOV,
    CMPCCXADD,
    CMPXCHG8B,
    CMPXCHG16B,
    #[strum(serialize = "CET-IBT")]
    CETIBT,
    MMX,
    #[strum(serialize = "MMX+")]
    MMXPlus,
    FEMMS,
    #[strum(serialize = "3dnow!")]
    _3DNow,
    #[strum(serialize = "3dnow!+")]
    _3DNowPlus,
    #[strum(serialize = "3dnow! Geode")]
    _3DNowGeode,
    SM3,
    SM4,
    SSE,
    SSE2,
    SSE3,
    SSSE3,
    #[strum(serialize = "SSE4.1")]
    SSE4_1, //
    #[strum(serialize = "SSE4.2")]
    SSE4_2, //
    SSE4A,
    #[strum(serialize = "AMX-TILE")]
    AMXTILE,
    #[strum(serialize = "AMX-COMPLEX")]
    AMXCOMPLEX,
    #[strum(serialize = "AMX-INT8")]
    AMXINT8,
    #[strum(serialize = "AMX-BF16")]
    AMXBF16,
    #[strum(serialize = "AMX-FP16")]
    AMXFP16,
    AVX,
    AVX2,
    XOP,
    FMA3,
    FMA4,
    F16C,
    PCLMULQDQ,
    AES,
    SHA,
    SHA512,
    RDRAND,
    RDSEED,
    RDPID,
    RDPMC,
    RDPRU,
    MOVBE,
    MOVDIRI,
    MOVDIR64B,
    POPCNT,
    LZCNT,
    BMI,
    BMI2,
    TBM,
    ADX,
    CLDEMOTE,
    CLFLUSH,
    CLFLUSHOPT,
    CLWB,
    LAHFSAHF,
    FSGSBASE,
    MCOMMIT,
    CLZERO,
    PREFETCH,
    PREFETCHI,
    PREFETCHW,
    PREFETCHWT1,
    MONITOR,
    MONITORX,
    SERIALIZE,
    WAITPKG,
    AVX512F,
    AVX512BW,
    AVX512DQ,
    AVX512VL,
    AVX512PF,
    AVX512ER,
    AVX512CD,
    #[strum(serialize = "AVX512-IFMA")]
    AVX512IFMA,
    #[strum(serialize = "AVX512-VPOPCNTDQ")]
    AVX512VPOPCNTDQ,
    #[strum(serialize = "AVX512-BF16")]
    AVX512BF16,
    #[strum(serialize = "AVX512-FP16")]
    AVX512FP16,
    #[strum(serialize = "AVX512-BITALG")]
    AVX512BITALG,
    #[strum(serialize = "AVX512-VBMI")]
    AVX512VBMI,
    #[strum(serialize = "AVX512-VBMI2")]
    AVX512VBMI2,
    #[strum(serialize = "AVX512-VNNI")]
    AVX512VNNI,
    #[strum(serialize = "AVX-VNNI")]
    AVXVNNI,
    #[strum(serialize = "AVX-VNNI-INT8")]
    AVXVNNIINT8,
    #[strum(serialize = "AVX-VNNI-INT16")]
    AVXVNNIINT16,
    #[strum(serialize = "AVX-NE-CONVERT")]
    AVXNECONVERT,
    #[strum(serialize = "AVX-IFMA")]
    AVXIFMA,
    // ARM
    A64,
}

// Operand
#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, Encode, BorrowDecode)]
pub struct Operand {
    pub type_: OperandType,
    pub input: Option<bool>,
    pub output: Option<bool>,
    pub extended_size: Option<usize>,
}

#[allow(non_camel_case_types)]
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    AsRefStr,
    Serialize,
    Deserialize,
    Encode,
    BorrowDecode,
)]
pub enum OperandType {
    #[strum(serialize = "1")]
    _1,
    #[strum(serialize = "3")]
    _3,
    imm4,
    imm8,
    imm16,
    imm32,
    imm64,
    al,
    cl,
    r8,
    r8l,
    ax,
    r16,
    r16l,
    eax,
    r32,
    r32l,
    rax,
    r64,
    mm,
    xmm0,
    xmm,
    #[strum(serialize = "xmm{k}")]
    xmm_k,
    #[strum(serialize = "xmm{k}{z}")]
    xmm_k_z,
    ymm,
    #[strum(serialize = "ymm{k}")]
    ymm_k,
    #[strum(serialize = "ymm{k}{z}")]
    ymm_k_z,
    zmm,
    #[strum(serialize = "zmm{k}")]
    zmm_k,
    #[strum(serialize = "zmm{k}{z}")]
    zmm_k_z,
    k,
    #[strum(serialize = "k{k}")]
    k_k,
    moffs32,
    moffs64,
    m,
    m8,
    m16,
    #[strum(serialize = "m16{k}")]
    m16_k,
    #[strum(serialize = "m16{k}{z}")]
    m16_k_z,
    m32,
    #[strum(serialize = "m32{k}")]
    m32_k,
    #[strum(serialize = "m32{k}{z}")]
    m32_k_z,
    #[strum(serialize = "m32/m16bcst")]
    m32_m16bcst,
    m64,
    #[strum(serialize = "m64{k}")]
    m64_k,
    #[strum(serialize = "m64{k}{z}")]
    m64_k_z,
    #[strum(serialize = "m64/m16bcst")]
    m64_m16bcst,
    m128,
    #[strum(serialize = "m128{k}")]
    m128_k,
    #[strum(serialize = "m128{k}{z}")]
    m128_k_z,
    m256,
    #[strum(serialize = "m256{k}")]
    m256_k,
    #[strum(serialize = "m256{k}{z}")]
    m256_k_z,
    m512,
    #[strum(serialize = "m512{k}")]
    m512_k,
    #[strum(serialize = "m512{k}{z}")]
    m512_k_z,
    #[strum(serialize = "m64/m32bcst")]
    m64_m32bcst,
    #[strum(serialize = "m128/m32bcst")]
    m128_m32bcst,
    #[strum(serialize = "m256/m32bcst")]
    m256_m32bcst,
    #[strum(serialize = "m512/m32bcst")]
    m512_m32bcst,
    #[strum(serialize = "m128/m16bcst")]
    m128_m16bcst,
    #[strum(serialize = "m128/m64bcst")]
    m128_m64bcst,
    #[strum(serialize = "m256/m16bcst")]
    m256_m16bcst,
    #[strum(serialize = "m256/m64bcst")]
    m256_m64bcst,
    #[strum(serialize = "m512/m16bcst")]
    m512_m16bcst,
    #[strum(serialize = "m512/m64bcst")]
    m512_m64bcst,
    vm32x,
    #[strum(serialize = "vm32x{k}")]
    vm32x_k,
    vm64x,
    #[strum(serialize = "vm64x{k}")]
    vm64xk,
    vm32y,
    #[strum(serialize = "vm32y{k}")]
    vm32yk_,
    vm64y,
    #[strum(serialize = "vm64y{k}")]
    vm64y_k,
    vm32z,
    #[strum(serialize = "vm32z{k}")]
    vm32z_k,
    vm64z,
    #[strum(serialize = "vm64z{k}")]
    vm64z_k,
    rel8,
    rel32,
    rel32m,
    #[strum(serialize = "{er}")]
    er,
    #[strum(serialize = "{sae}")]
    sae,
    sibmem,
    tmm,

    // Avr operand types
    Rd,
    Rr,
    X,
    Y,
    Z,
    #[strum(serialize = "-X")]
    NegX,
    #[strum(serialize = "-Y")]
    NegY,
    #[strum(serialize = "-Z")]
    NegZ,
    #[strum(serialize = "X+")]
    XPlus,
    #[strum(serialize = "Y+")]
    YPlus,
    #[strum(serialize = "Z+")]
    ZPlus,
    #[strum(serialize = "Y+q")]
    YPlusQ,
    #[strum(serialize = "Z+q")]
    ZPlusQ,
    A,
    K,
    // `k` is already covered
    s,
    b,
}

// lsp types

/// Represents a text cursor between characters, pointing at the next character in the buffer.
pub type Column = usize;

/// Stores a tree-sitter tree and it associated parser for a given source file
pub struct TreeEntry {
    pub tree: Option<Tree>,
    pub parser: Parser,
}

/// Associates URIs with their corresponding tree-sitter tree and parser
pub type TreeStore = BTreeMap<Uri, TreeEntry>;

#[derive(Default)]
pub struct DocumentStore {
    pub tree_store: TreeStore,
    pub text_store: TextDocuments,
}

impl DocumentStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tree_store: TreeStore::new(),
            text_store: TextDocuments::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CompletionItems {
    pub instructions: Vec<(Arch, CompletionItem)>,
    pub registers: Vec<(Arch, CompletionItem)>,
    pub directives: Vec<(Assembler, CompletionItem)>,
}

impl CompletionItems {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            instructions: Vec::new(),
            registers: Vec::new(),
            directives: Vec::new(),
        }
    }
}

/// Struct to store all documentation the server uses to service user requests
#[derive(Default, Debug)]
pub struct ServerStore {
    /// Links names of instructions, registers, and directives to their documentation
    pub names_to_info: NameToInfoMaps,
    /// `Completion` items for instructions, registers, and directives
    pub completion_items: CompletionItems,
    /// Compilation database loaded from `compile_commands.json` or `compile_flags.txt`
    pub compile_commands: CompilationDatabase,
    /// Include directories
    pub include_dirs: HashMap<SourceFile, Vec<PathBuf>>,
}
