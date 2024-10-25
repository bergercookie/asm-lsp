use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Display,
    path::PathBuf,
    str::FromStr,
};

use log::{info, warn};
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, Display, EnumString};
use tree_sitter::{Parser, Tree};

use crate::{
    populate_name_to_directive_map, populate_name_to_instruction_map, populate_name_to_register_map,
};

// Instruction
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Instruction {
    pub name: String,
    pub summary: String,
    pub forms: Vec<InstructionForm>,
    pub asm_templates: Vec<String>,
    pub aliases: Vec<InstructionAlias>,
    pub url: Option<String>,
    pub arch: Option<Arch>,
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
        let arch = None;

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
        let header: String;
        if let Some(arch) = &self.arch {
            header = format!("{} [{}]", &self.name, arch.as_ref());
        } else {
            header = self.name.clone();
        }

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

// InstructionForm
#[derive(Default, Eq, PartialEq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct InstructionForm {
    // --- Gas/Go-Specific Information ---
    pub gas_name: Option<String>,
    pub go_name: Option<String>,
    pub mmx_mode: Option<MMXMode>,
    pub xmm_mode: Option<XMMMode>,
    pub cancelling_inputs: Option<bool>,
    pub nacl_version: Option<u8>,
    pub nacl_zero_extends_outputs: Option<bool>,
    pub operands: Vec<Operand>,
    // --- Z80-Specific Information ---
    pub z80_name: Option<String>,
    pub z80_form: Option<String>,
    pub z80_opcode: Option<String>,
    pub z80_timing: Option<Z80Timing>,
    // --- Assembler/Architecture Agnostic Info ---
    pub isa: Option<ISA>,
    pub urls: Vec<String>,
}

impl std::fmt::Display for InstructionForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        if let Some(val) = &self.gas_name {
            s += &format!("*GAS*: {val} | ");
        }
        if let Some(val) = &self.go_name {
            s += &format!("*GO*: {val} | ");
        }
        if let Some(val) = &self.z80_form {
            s += &format!("*Z80*: {val} | ");
        }

        if let Some(val) = &self.mmx_mode {
            s += &(format!("*MMX*: {} | ", val.as_ref()));
        }
        if let Some(val) = &self.xmm_mode {
            s += &(format!("*XMM*: {} | ", val.as_ref()));
        }
        if let Some(val) = &self.z80_opcode {
            if val.contains(',') {
                s += &format!("*Opcodes*: {val} | ");
            } else {
                s += &format!("*Opcode*: {val} | ");
            }
        }

        // cancelling inputs
        // nacl_version
        // nacl_zero_extends_outputs

        // ISA
        if let Some(val) = &self.isa {
            s += &format!("*ISA*: {} | ", val.as_ref());
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
                        s += &format!(" input = {input:<5} ");
                    }
                    if let Some(output) = op.output {
                        s += &format!(" output = {output:<5}");
                    }
                    if let Some(extended_size) = op.extended_size {
                        s += &format!(" extended-size = {extended_size}");
                    }

                    s.trim_end().to_owned()
                })
                .collect::<Vec<String>>()
                .join("\n")
        };

        s += &operands_str;

        if let Some(ref timing) = self.z80_timing {
            s += &format!("\n  + {timing}");
        }

        for url in &self.urls {
            s += &format!("\n  + More info: {url}\n");
        }

        write!(f, "{s}")?;
        Ok(())
    }
}

// InstructionAlias
#[derive(Default, Eq, PartialEq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct InstructionAlias {
    pub title: String,
    pub summary: String,
    pub asm_templates: Vec<String>,
}

impl std::fmt::Display for InstructionAlias {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = self.title.clone();

        if !self.summary.is_empty() {
            s += &format!("\n {}", self.summary);
        }

        for template in &self.asm_templates {
            s += &format!("\n + `{template}`");
        }

        write!(f, "{s}")?;
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, Serialize, Deserialize)]
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

#[derive(Default, Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
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

// Directive
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Directive {
    pub name: String,
    pub signatures: Vec<String>,
    pub description: String,
    pub deprecated: bool,
    pub url: Option<String>,
    pub assembler: Option<Assembler>,
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
        let assembler = None;

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
        let header: String;
        if let Some(assembler) = &self.assembler {
            header = format!(
                "{} [{}]{}",
                &self.name,
                assembler.as_ref(),
                if self.deprecated {
                    "\n**DEPRECATED**"
                } else {
                    ""
                }
            );
        } else {
            header = self.name.clone();
        }

        let mut v: Vec<&str> = vec![&header, &self.description, "\n"];

        // signature(s)
        let mut sigs = String::new();
        for sig in &self.signatures {
            sigs += &format!("- {sig}\n");
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Register {
    pub name: String,
    pub description: Option<String>,
    pub reg_type: Option<RegisterType>,
    pub width: Option<RegisterWidth>,
    pub flag_info: Vec<RegisterBitInfo>,
    pub arch: Option<Arch>,
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
        let arch = None;
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
        let header: String;
        if let Some(arch) = &self.arch {
            header = format!("{} [{}]", &self.name.to_uppercase(), arch.as_ref());
        } else {
            header = self.name.to_uppercase();
        }

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString, AsRefStr, Serialize, Deserialize)]
pub enum XMMMode {
    SSE,
    AVX,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, EnumString, AsRefStr, Serialize, Deserialize)]
pub enum MMXMode {
    FPU,
    MMX,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, EnumString, AsRefStr, Serialize, Deserialize)]
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
    ARM64,
    #[strum(serialize = "riscv")]
    #[serde(rename = "riscv")]
    RISCV,
    #[strum(serialize = "z80")]
    #[serde(rename = "z80")]
    Z80,
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
        match self {
            Self::X86 => {
                // create a map of &Register_name -> &Register - Use that in user queries
                // The Register(s) themselves are stored in a vector and we only keep references to the
                // former map
                let start = std::time::Instant::now();
                let regs_x86 = include_bytes!("serialized/registers/x86");
                let x86_registers =
                    bincode::deserialize(regs_x86)
                    .expect("Error deserializing x86 registers;\nRegenerate serialized data with regenerate.sh");
                info!(
                    "x86 register set loaded in {}ms",
                    start.elapsed().as_millis()
                );

                populate_name_to_register_map(Self::X86, &x86_registers, names_to_registers);
            }
            Self::X86_64 => {
                let start = std::time::Instant::now();
                let regs_x86_64 = include_bytes!("serialized/registers/x86_64");
                let x86_64_registers = bincode::deserialize(regs_x86_64)
                    .expect("Error deserializing x86-64 registers;\nRegenerate serialized data with regenerate.sh");
                info!(
                    "x86-64 register set loaded in {}ms",
                    start.elapsed().as_millis()
                );

                populate_name_to_register_map(Self::X86_64, &x86_64_registers, names_to_registers);
            }
            Self::X86_AND_X86_64 => {
                let start = std::time::Instant::now();

                let regs_x86 = include_bytes!("serialized/registers/x86");
                let x86_registers =
                    bincode::deserialize(regs_x86)
                    .expect("Error deserializing x86 registers;\nRegenerate serialized data with regenerate.sh");

                let regs_x86_64 = include_bytes!("serialized/registers/x86_64");
                let x86_64_registers = bincode::deserialize(regs_x86_64)
                    .expect("Error deserializing x86-64 registers;\nRegenerate serialized data with regenerate.sh");

                populate_name_to_register_map(Self::X86, &x86_registers, names_to_registers);
                populate_name_to_register_map(Self::X86_64, &x86_64_registers, names_to_registers);

                info!(
                    "x86/x86-64 register set loaded in {}ms",
                    start.elapsed().as_millis()
                );
            }
            Self::ARM => {
                let start = std::time::Instant::now();
                let regs_arm = include_bytes!("serialized/registers/arm");
                let arm_registers =
                    bincode::deserialize(regs_arm)
                    .expect("Error deserializing arm registers;\nRegenerate serialized data with regenerate.sh");
                info!(
                    "arm register set loaded in {}ms",
                    start.elapsed().as_millis()
                );

                populate_name_to_register_map(Self::ARM, &arm_registers, names_to_registers);
            }
            Self::ARM64 => {
                let start = std::time::Instant::now();
                let regs_arm64 = include_bytes!("serialized/registers/arm64");
                let arm64_registers = bincode::deserialize(regs_arm64)
                    .expect("Error deserializing arm64 registers;\nRegenerate serialized data with regenerate.sh");
                populate_name_to_register_map(Self::ARM64, &arm64_registers, names_to_registers);
                info!(
                    "arm register set loaded in {}ms",
                    start.elapsed().as_millis()
                );
            }
            Self::RISCV => {
                let start = std::time::Instant::now();
                let regs_riscv = include_bytes!("serialized/registers/riscv");
                let riscv_registers =
                    bincode::deserialize(regs_riscv)
                    .expect("Error deserializing riscv registers;\nRegenerate serialized data with regenerate.sh");
                info!(
                    "riscv register set loaded in {}ms",
                    start.elapsed().as_millis()
                );

                populate_name_to_register_map(Self::RISCV, &riscv_registers, names_to_registers);
            }
            Self::Z80 => {
                let start = std::time::Instant::now();
                let regs_z80 = include_bytes!("serialized/registers/z80");
                let z80_registers =
                    bincode::deserialize(regs_z80)
                    .expect("Error deserializing z80 registers;\nRegenerate serialized data with regenerate.sh");
                info!(
                    "z80 register set loaded in {}ms",
                    start.elapsed().as_millis()
                );
                populate_name_to_register_map(Self::Z80, &z80_registers, names_to_registers);
            }
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
        names_to_instructions: &mut HashMap<(Self, String), Instruction>,
    ) {
        match self {
            Self::X86 => {
                let start = std::time::Instant::now();
                let x86_instrs = include_bytes!("serialized/opcodes/x86");
                let x86_instructions = bincode::deserialize::<Vec<Instruction>>(x86_instrs)
                    .expect("Error deserializing x86 instructions;\nRegenerate serialized data with regenerate.sh");
                info!(
                    "x86 instruction set loaded in {}ms",
                    start.elapsed().as_millis()
                );

                populate_name_to_instruction_map(
                    Self::X86,
                    &x86_instructions,
                    names_to_instructions,
                );
            }
            Self::X86_64 => {
                let start = std::time::Instant::now();
                let x86_64_instrs = include_bytes!("serialized/opcodes/x86_64");
                let x86_64_instructions = bincode::deserialize::<Vec<Instruction>>(x86_64_instrs)
                    .expect("Error deserializing x86_64 instructions;\nRegenerate serialized data with regenerate.sh");
                info!(
                    "x86-64 instruction set loaded in {}ms",
                    start.elapsed().as_millis()
                );

                populate_name_to_instruction_map(
                    Self::X86_64,
                    &x86_64_instructions,
                    names_to_instructions,
                );
            }
            Self::X86_AND_X86_64 => {
                let start = std::time::Instant::now();

                let x86_instrs = include_bytes!("serialized/opcodes/x86");
                let x86_instructions = bincode::deserialize::<Vec<Instruction>>(x86_instrs)
                    .expect("Error deserializing x86 instructions;\nRegenerate serialized data with regenerate.sh");

                let x86_64_instrs = include_bytes!("serialized/opcodes/x86_64");
                let x86_64_instructions = bincode::deserialize::<Vec<Instruction>>(x86_64_instrs)
                    .expect("Error deserializing x86_64 instructions;\nRegenerate serialized data with regenerate.sh");

                populate_name_to_instruction_map(
                    Self::X86,
                    &x86_instructions,
                    names_to_instructions,
                );

                populate_name_to_instruction_map(
                    Self::X86_64,
                    &x86_64_instructions,
                    names_to_instructions,
                );

                info!(
                    "x86/x86-64 instruction sets loaded in {}ms",
                    start.elapsed().as_millis()
                );
            }
            Self::ARM => {
                let start = std::time::Instant::now();
                let arm_instrs = include_bytes!("serialized/opcodes/arm");
                let arm_instructions = bincode::deserialize::<Vec<Instruction>>(arm_instrs)
                    .expect("Error deserializing arm32 instructions;\nRegenerate serialized data with regenerate.sh");
                info!(
                    "arm instruction set loaded in {}ms",
                    start.elapsed().as_millis()
                );

                populate_name_to_instruction_map(
                    Self::ARM,
                    &arm_instructions,
                    names_to_instructions,
                );
            }
            Self::ARM64 => {
                let start = std::time::Instant::now();
                // TODO: change to arm64 after arm32 has been added
                let arm_instrs = include_bytes!("serialized/opcodes/arm");
                // NOTE: Actually, the arm file are all arm64 so we needed to get
                // the arm32 versions then do the below
                // NOTE: No need to filter these instructions by assembler
                // like we do for x86/x86_64, as our ARM docs don't contain any
                // assembler-specific information (yet)
                let arm64_instructions = bincode::deserialize::<Vec<Instruction>>(arm_instrs)
                    .expect("Error deserializing arm64 instructions;\nRegenerate serialized data with regenerate.sh");
                populate_name_to_instruction_map(
                    Self::ARM64,
                    &arm64_instructions,
                    names_to_instructions,
                );
                info!(
                    "arm64 instruction set loaded in {}ms",
                    start.elapsed().as_millis()
                );
            }
            Self::RISCV => {
                let start = std::time::Instant::now();
                let riscv_instrs = include_bytes!("serialized/opcodes/riscv");
                // NOTE: No need to filter these instructions by assembler like we do for
                // x86/x86_64, as our RISCV docs don't contain any assembler-specific information (yet)
                let riscv_instructions = bincode::deserialize::<Vec<Instruction>>(riscv_instrs).expect("Error deserializing riscv instructions;\nRegenerate serialized data with regenerate.sh");
                info!(
                    "riscv instruction set loaded in {}ms",
                    start.elapsed().as_millis()
                );

                populate_name_to_instruction_map(
                    Self::RISCV,
                    &riscv_instructions,
                    names_to_instructions,
                );
            }
            Self::Z80 => {
                let start = std::time::Instant::now();
                let z80_instrs = include_bytes!("serialized/opcodes/z80");
                let z80_instructions = bincode::deserialize::<Vec<Instruction>>(z80_instrs)
                    .expect("Error deserializing z80 instructions;\nRegenerate serialized data with regenerate.sh");
                info!(
                    "z80 instruction set loaded in {}ms",
                    start.elapsed().as_millis()
                );

                populate_name_to_instruction_map(
                    Self::Z80,
                    &z80_instructions,
                    names_to_instructions,
                );
            }
            Self::None => unreachable!(),
        }
    }
}

impl Default for Arch {
    fn default() -> Self {
        if cfg!(target_arch = "x86") {
            info!("Detected host arch as x86");
            Self::X86
        } else if cfg!(target_arch = "arm") {
            info!("Detected host arch as arm");
            Self::ARM
        } else if cfg!(target_arch = "aarch64") {
            info!("Detected host arch as aarch64");
            Self::ARM64
        } else if cfg!(target_arch = "riscv32") {
            info!("Detected host arch as riscv32");
            Self::RISCV
        } else if cfg!(target_arch = "riscv64") {
            info!("Detected host arch as riscv64");
            Self::RISCV
        } else {
            if cfg!(target_arch = "x86_64") {
                info!("Detected host arch as x86-64");
            } else {
                info!("Failed to detect host arch, defaulting to x86-64");
            }
            Self::X86_64 // somewhat arbitrary default
        }
    }
}

impl std::fmt::Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::X86 => write!(f, "x86")?,
            Self::X86_64 => write!(f, "x86-64")?,
            Self::X86_AND_X86_64 => write!(f, "x86/x86-64")?,
            Self::ARM => write!(f, "arm")?,
            Self::ARM64 => write!(f, "arm64")?,
            Self::Z80 => write!(f, "z80")?,
            Self::RISCV => write!(f, "riscv")?,
            Self::None => write!(f, "None")?,
        }
        Ok(())
    }
}

// All of the `#[serde(rename = "...")] allows for both lower case and the literal
// enum representations of the values to be deserialized
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
)]
pub enum Assembler {
    #[default]
    #[strum(serialize = "gas")]
    #[serde(rename = "gas")]
    Gas,
    #[strum(serialize = "go")]
    #[serde(rename = "go")]
    Go,
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
        match self {
            Self::Gas => {
                let start = std::time::Instant::now();
                let gas_dirs = include_bytes!("serialized/directives/gas");
                let gas_directives =
                    bincode::deserialize(gas_dirs).expect("Error deserializing Gas directives\nRegenerate serialized data with regenerate.sh");
                info!(
                    "Gas directive set loaded in {}ms",
                    start.elapsed().as_millis()
                );

                populate_name_to_directive_map(Self::Gas, &gas_directives, names_to_directives);
            }
            Self::Masm => {
                let start = std::time::Instant::now();
                let masm_dirs = include_bytes!("serialized/directives/masm");
                let masm_directives =
                    bincode::deserialize(masm_dirs).expect("Error deserializing Masm directives\nRegenerate serialized data with regenerate.sh");
                info!(
                    "MASM directive set loaded in {}ms",
                    start.elapsed().as_millis()
                );

                populate_name_to_directive_map(Self::Masm, &masm_directives, names_to_directives);
            }
            Self::Nasm => {
                let start = std::time::Instant::now();
                let nasm_dirs = include_bytes!("serialized/directives/nasm");
                let nasm_directives =
                    bincode::deserialize(nasm_dirs).expect("Error deserializing Nasm directives\nRegenerate serialized data with regenerate.sh");
                info!(
                    "Nasm directive set loaded in {}ms",
                    start.elapsed().as_millis()
                );

                populate_name_to_directive_map(Self::Nasm, &nasm_directives, names_to_directives);
            }
            Self::Go => {
                warn!("There is currently no Go-specific assembler documentation");
            }
            Self::None => unreachable!(),
        }
    }
}

#[derive(
    Debug, Hash, PartialEq, Eq, Clone, Copy, EnumString, AsRefStr, Display, Serialize, Deserialize,
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
    Debug, Hash, PartialEq, Eq, Clone, Copy, EnumString, AsRefStr, Display, Serialize, Deserialize,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Default, Deserialize)]
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
            s += &format!(", PAE: {}", self.pae);
        }
        if !self.long_mode.is_empty() {
            s += &format!(", Long Mode: {}", self.long_mode);
        }

        write!(f, "{s}")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        #[allow(irrefutable_let_patterns)]
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
    /// Will panic if `req_uri` cannot be canonicalized
    pub fn get_config<'a>(&'a self, req_uri: &'a Uri) -> &'a Config {
        #[allow(irrefutable_let_patterns)]
        let Ok(req_path) = PathBuf::from_str(req_uri.path().as_str()) else {
            unreachable!()
        };
        let request_path = match req_path.canonicalize() {
            Ok(path) => path,
            Err(e) => panic!("Invalid request path: \"{}\" - {e}", req_path.display()),
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

    /// Sets the `client` field of the default config and all project configs
    pub fn set_client(&mut self, client: LspClient) {
        if let Some(ref mut root) = self.default_config {
            root.client = Some(client);
        }

        if let Some(ref mut projects) = self.projects {
            for project in projects {
                project.config.client = Some(client);
            }
        }
    }

    #[must_use]
    pub fn is_isa_enabled(&self, isa: Arch) -> bool {
        if let Some(ref root) = self.default_config {
            if root.is_isa_enabled(isa) {
                return true;
            }
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
        if let Some(ref root) = self.default_config {
            if root.is_assembler_enabled(assembler) {
                return true;
            }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    // path to a directory or source file on which this config applies
    // can be relative to the server's root directory, or absolute
    pub path: PathBuf,
    #[serde(flatten)]
    pub config: Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: Option<String>,
    pub assembler: Assembler,
    pub instruction_set: Arch,
    pub opts: Option<ConfigOptions>,
    #[serde(skip)]
    pub client: Option<LspClient>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: Some(String::from("0.1")),
            assembler: Assembler::default(),
            instruction_set: Arch::default(),
            opts: Some(ConfigOptions::default()),
            client: None,
        }
    }
}

impl Config {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LspClient {
    Helix,
}

// Instruction Set Architecture
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumString, AsRefStr, Serialize, Deserialize)]
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
#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Operand {
    pub type_: OperandType,
    pub input: Option<bool>,
    pub output: Option<bool>,
    pub extended_size: Option<usize>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString, AsRefStr, Serialize, Deserialize)]
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
