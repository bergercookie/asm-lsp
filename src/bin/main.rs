use log::{info, warn, error};
use quick_xml::events::Event;
use quick_xml::events::attributes::Attribute;
use quick_xml::Reader;
use url::Url;
use std::str;

// Instruction ------------------------------------------------------------------------------------
#[derive(Debug)]
pub struct Instruction {
    pub name: String,
    pub summary: String,
    pub forms: Vec<InstructionForm>,
    pub url: Option<Url>,
    // TODO - Add example?
    // TODO - Add URL?
    // TODO - Have an independent script for parsing the URLs from various websites
    //        https://www.felixcloutier.com/x86/
}

impl Default for Instruction {
    fn default() -> Self {
        let name = String::new();
        let summary = String::new();
        let forms = vec![];
        let url = None;

        Self {
            name,
            summary,
            forms,
            url
        }
    }
}

impl Instruction {
    /// Add a new form at the current instruction
    fn push_form(&mut self, form: InstructionForm) {
        self.forms.push(form);
    }
}

// InstructionForm --------------------------------------------------------------------------------
#[derive(Default, Debug)]
pub struct InstructionForm {
    pub gas_name: Option<String>,
    pub go_name: Option<String>,
    pub cancelling_inputs: Option<bool>,
    pub nacl_version: Option<u8>,
    pub nacl_zero_extends_outputs: Option<bool>,
    pub xmm_version: Option<XMMVersion>,
}

impl InstructionForm {
}

// helper structs ---------------------------------------------------------------------------------
#[derive(Debug)]
pub enum InstructionSet {
    X86(Vec<Instruction>),
    X86_64(Vec<Instruction>),
    K1OM(Vec<Instruction>),
}

impl InstructionSet {
    fn push(&mut self, instruction: Instruction) {
        match self {
           InstructionSet::X86(vec)  => vec.push(instruction),
           InstructionSet::X86_64(vec)  => vec.push(instruction),
           InstructionSet::K1OM(vec)  => vec.push(instruction),
        }
    }
}

#[derive(Debug)]
pub enum XMMVersion {
    SSE,
    AVX,
}

// main -------------------------------------------------------------------------------------------
pub fn main() {
    // initialisation -----------------------------------------------------------------------------
    env_logger::init();

    let mut buf = Vec::new();

    // initialise the instruction set
    let mut instruction_set = vec![];


    // Embed the codes in the binary
    let opcodes_x86 = include_str!("../../opcodes/x86.xml");
    // let opcodes_x86_64 = include_str!("../../opcodes/x86_64.xml");
    // let opcodes_k1om = include_str!("../../opcodes/k1om.xml");

    // iterate through the XML --------------------------------------------------------------------
    let mut reader = Reader::from_str(opcodes_x86);
    reader.trim_text(true);

    // ref to the instruction that's currently under construction
    let mut curr_instruction: &mut Instruction;
    // let mut instruction_form_cursor: &InstructionForm;

    loop {
        match reader.read_event(&mut buf) {
            // start event ------------------------------------------------------------------------
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    b"Instruction" => {
                        // start of a new instruction
                        instruction_set.push(Instruction::default());
                        curr_instruction = instruction_set.last_mut().unwrap();

                        // iterate over the attributes
                        for attr in e.attributes() {
                            let Attribute {key, value} =  attr.unwrap();
                            match str::from_utf8(key).unwrap() {
                                "name" => {
                                    unsafe {curr_instruction.name = String::from_utf8_unchecked(value.to_vec()); }
                                },
                                "summary" => {
                                    unsafe {curr_instruction.summary = String::from_utf8_unchecked(value.to_vec()); }
                                }
                                _ => {}
                            }
                        }
                    },
                    b"InstructionForm" => {
                        // TODO add to currently open instruction
                        // open_instruction_form = InstructionForm::default();


				// <xs:attribute name="gas-name" type="xs:string" use="required" />
				//     <xs:attribute name="go-name" type="xs:string" />
				//     <xs:attribute name="mmx-mode" type="MMXMode" />
				//     <xs:attribute name="xmm-mode" type="XMMMode" />
				//     <xs:attribute name="cancelling-inputs" type="xs:boolean" />
				//     <xs:attribute name="nacl-version" type="NaClVersion" />
				//     <xs:attribute name="nacl-zero-extends-outputs" type="xs:boolean" />

                    },
                    _ => (), // unknown event
                }
            }
            // end event --------------------------------------------------------------------------
            Ok(Event::End(ref e)) => {
                match e.name() {
                    b"Instruction" => {
                        // finish instruction
                    },
                    b"InstructionForm" => {
                        // TODO add to currently open instruction
                        // open_instruction_form = InstructionForm::default();
                    },
                    _ => (), // unknown event
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // rest of events that we don't consider
        }
    }

    // provide a URL example page


    println!("instruction_set: {:#?}", instruction_set);

}
