#[cfg(test)]
mod tests {
    use std::sync::{Mutex, Once, OnceLock};

    use anyhow::Result;
    use lsp_textdocument::FullTextDocument;
    use lsp_types::{
        HoverContents, MarkupContent, MarkupKind, Position, TextDocumentIdentifier,
        TextDocumentPositionParams, Url,
    };
    // use tree_sitter::Parser;

    use crate::{
        get_hover_resp, get_word_from_pos_params, instr_filter_targets, populate_directives,
        populate_name_to_directive_map, populate_name_to_instruction_map,
        populate_name_to_register_map, populate_registers, x86_parser::get_cache_dir, Arch,
        Assembler, Assemblers, Instruction, InstructionSets, NameToDirectiveMap,
        NameToInstructionMap, NameToRegisterMap, TargetConfig,
    };

    // TODO: Add include_dirs
    #[derive(Debug)]
    struct GlobalVars {
        names_to_instructions: NameToInstructionMap,
        names_to_registers: NameToRegisterMap,
        names_to_directives: NameToDirectiveMap,
        // instr_completion_items: Vec<CompletionItem>,
        // reg_completion_items: Vec<CompletionItem>,
        // directive_completion_items: Vec<CompletionItem>,
    }

    impl GlobalVars {
        fn new() -> Self {
            Self {
                names_to_instructions: NameToInstructionMap::new(),
                names_to_registers: NameToRegisterMap::new(),
                names_to_directives: NameToDirectiveMap::new(),
                // instr_completion_items: Vec::new(),
                // reg_completion_items: Vec::new(),
                // directive_completion_items: Vec::new(),
            }
        }
    }

    // TODO: Look into a way to cleanly handle multiple GLOBALS, each representing the server
    // setup from a different configuration
    static SETUP: Once = Once::new();
    static GLOBALS: OnceLock<Mutex<GlobalVars>> = OnceLock::new();

    fn init_test_store() -> Result<GlobalVars> {
        let mut store = GlobalVars::new();
        let target_config = TargetConfig {
            version: "0.1".to_string(),
            assemblers: Assemblers {
                gas: true,
                go: true,
                z80: true,
            },
            instruction_sets: InstructionSets {
                x86: true,
                x86_64: true,
                z80: true,
            },
        };

        let mut x86_cache_path = get_cache_dir().unwrap();
        x86_cache_path.push("x86_instr_docs.html");
        if x86_cache_path.is_file() {
            std::fs::remove_file(&x86_cache_path).unwrap();
        }

        let x86_instructions: Vec<Instruction> = {
            let xml_conts_x86 = include_str!("../opcodes/x86.xml");
            crate::populate_instructions(xml_conts_x86)?
                .into_iter()
                .map(|instruction| instr_filter_targets(&instruction, &target_config))
                .filter(|instruction| !instruction.forms.is_empty())
                .collect()
        };

        populate_name_to_instruction_map(
            Arch::X86,
            &x86_instructions,
            &mut store.names_to_instructions,
        );

        let x86_64_instructions: Vec<Instruction> = {
            let xml_conts_x86_64 = include_str!("../opcodes/x86_64.xml");
            crate::populate_instructions(xml_conts_x86_64)?
                .into_iter()
                .map(|instruction| instr_filter_targets(&instruction, &target_config))
                .filter(|instruction| !instruction.forms.is_empty())
                .collect()
        };

        populate_name_to_instruction_map(
            Arch::X86_64,
            &x86_64_instructions,
            &mut store.names_to_instructions,
        );

        let z80_instructions: Vec<Instruction> = {
            let xml_conts_z80 = include_str!("../opcodes/z80.xml");
            crate::populate_instructions(xml_conts_z80)?
                .into_iter()
                .map(|instruction| instr_filter_targets(&instruction, &target_config))
                .filter(|instruction| !instruction.forms.is_empty())
                .collect()
        };

        populate_name_to_instruction_map(
            Arch::Z80,
            &z80_instructions,
            &mut store.names_to_instructions,
        );

        let x86_registers = {
            let xml_conts_regs_x86 = include_str!("../registers/x86.xml");
            populate_registers(xml_conts_regs_x86)?
                .into_iter()
                .collect()
        };

        populate_name_to_register_map(Arch::X86, &x86_registers, &mut store.names_to_registers);

        let x86_64_registers = {
            let xml_conts_regs_x86_64 = include_str!("../registers/x86_64.xml");
            populate_registers(xml_conts_regs_x86_64)?
                .into_iter()
                .collect()
        };

        populate_name_to_register_map(
            Arch::X86_64,
            &x86_64_registers,
            &mut store.names_to_registers,
        );

        let z80_registers = {
            let xml_conts_regs_z80 = include_str!("../registers/z80.xml");
            populate_registers(xml_conts_regs_z80)?
                .into_iter()
                .collect()
        };

        populate_name_to_register_map(Arch::Z80, &z80_registers, &mut store.names_to_registers);

        let gas_directives = {
            let xml_conts_gas = include_str!("../directives/gas_directives.xml");
            populate_directives(xml_conts_gas)?.into_iter().collect()
        };

        populate_name_to_directive_map(
            Assembler::Gas,
            &gas_directives,
            &mut store.names_to_directives,
        );

        return Ok(store);
    }

    fn prepare_globals() {
        SETUP.call_once(|| {
            GLOBALS.set(Mutex::new(init_test_store().unwrap())).unwrap();
        });
    }

    fn test_hover(source: &str, expected: &str) {
        prepare_globals();

        let globals = GLOBALS
            .get()
            .expect("global store not initialized")
            .lock()
            .expect("global store mutex poisoned");

        let source_code = source.replace("<cursor>", "");
        let curr_doc = Some(FullTextDocument::new(
            "asm".to_string(),
            1,
            source_code.clone(),
        ));

        let mut position: Option<Position> = None;
        for (line_num, line) in source.lines().enumerate() {
            if let Some((idx, _)) = line.match_indices("<cursor>").next() {
                position = Some(Position {
                    line: line_num as u32,
                    character: idx as u32,
                });
                break;
            }
        }

        let pos_params = TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: Url::parse("file://").unwrap(),
            },
            position: position.expect("No <cursor> marker found"),
        };

        let (word, file_word) = if let Some(ref doc) = curr_doc {
            (
                // get the word under the cursor
                get_word_from_pos_params(doc, &pos_params, ""),
                // treat the word under the cursor as a filename and grab it as well
                get_word_from_pos_params(doc, &pos_params, "."),
            )
        } else {
            panic!("No document");
        };

        let resp = get_hover_resp(
            &word,
            &file_word,
            &globals.names_to_instructions,
            &globals.names_to_registers,
            &globals.names_to_directives,
            &Vec::new(),
        )
        .unwrap();

        if let HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: resp_text,
        }) = resp.contents
        {
            let cleaned = resp_text.replace("\n\n\n", "\n\n"); // not sure what's going on here...
            assert_eq!(expected, cleaned);
        } else {
            panic!("Invalid hover response contents: {:?}", resp.contents);
        }
    }

    #[test]
    fn handle_hover_x86_x86_64_it_provides_instr_info_no_args() {
        test_hover(
            "<cursor>MOVLPS",
            "MOVLPS [x86]
Move Low Packed Single-Precision Floating-Point Values

## Forms

- *GAS*: movlps | *GO*: MOVLPS | *XMM*: SSE | *ISA*: SSE

  + [xmm]    input = true   output = true
  + [m64]    input = true   output = false
- *GAS*: movlps | *GO*: MOVLPS | *XMM*: SSE | *ISA*: SSE

  + [m64]    input = false  output = true
  + [xmm]    input = true   output = false

MOVLPS [x86-64]
Move Low Packed Single-Precision Floating-Point Values

## Forms

- *GAS*: movlps | *GO*: MOVLPS | *XMM*: SSE | *ISA*: SSE

  + [xmm]    input = true   output = true
  + [m64]    input = true   output = false
- *GAS*: movlps | *GO*: MOVLPS | *XMM*: SSE | *ISA*: SSE

  + [m64]    input = false  output = true
  + [xmm]    input = true   output = false",
        );
    }
    #[test]
    fn handle_hover_x86_x86_64_it_provides_instr_info_one_reg_arg() {
        test_hover(
            "push<cursor>q	%rbp",
            "PUSH [x86]
Push Value Onto the Stack

## Forms

- *GAS*: pushq

  + [imm8]   extended-size = 4
- *GAS*: pushq

  + [imm32]
- *GAS*: pushw | *GO*: PUSHW

  + [r16]    input = true   output = false
- *GAS*: pushl | *GO*: PUSHL

  + [r32]    input = true   output = false
- *GAS*: pushw | *GO*: PUSHW

  + [m16]    input = true   output = false
- *GAS*: pushl | *GO*: PUSHL

  + [m32]    input = true   output = false

PUSH [x86-64]
Push Value Onto the Stack

## Forms

- *GAS*: pushq | *GO*: PUSHQ

  + [imm8]   extended-size = 8
- *GAS*: pushq | *GO*: PUSHQ

  + [imm32]  extended-size = 8
- *GAS*: pushw | *GO*: PUSHW

  + [r16]    input = true   output = false
- *GAS*: pushq | *GO*: PUSHQ

  + [r64]    input = true   output = false
- *GAS*: pushw | *GO*: PUSHW

  + [m16]    input = true   output = false
- *GAS*: pushq | *GO*: PUSHQ

  + [m64]    input = true   output = false",
        );
    }
    #[test]
    fn handle_hover_x86_x86_64_it_provides_instr_info_two_reg_args() {
        test_hover(
            "	m<cursor>ovq	%rsp, %rbp",
            "MOVQ [x86]
Move Quadword

## Forms

- *GAS*: movq | *GO*: MOVQ | *MMX*: MMX | *ISA*: MMX

  + [mm]     input = false  output = true
  + [mm]     input = true   output = false
- *GAS*: movq | *GO*: MOVQ | *MMX*: MMX | *ISA*: MMX

  + [mm]     input = false  output = true
  + [m64]    input = true   output = false
- *GAS*: movq | *GO*: MOVQ | *XMM*: SSE | *ISA*: SSE2

  + [xmm]    input = false  output = true
  + [xmm]    input = true   output = false
- *GAS*: movq | *GO*: MOVQ | *XMM*: SSE | *ISA*: SSE2

  + [xmm]    input = false  output = true
  + [m64]    input = true   output = false
- *GAS*: movq | *GO*: MOVQ | *MMX*: MMX | *ISA*: MMX

  + [m64]    input = false  output = true
  + [mm]     input = true   output = false
- *GAS*: movq | *GO*: MOVQ | *XMM*: SSE | *ISA*: SSE2

  + [m64]    input = false  output = true
  + [xmm]    input = true   output = false

MOVQ [x86-64]
Move Quadword

## Forms

- *GAS*: movq | *GO*: MOVQ | *MMX*: MMX | *ISA*: MMX

  + [r64]    input = false  output = true
  + [mm]     input = true   output = false
- *GAS*: movq | *GO*: MOVQ | *XMM*: SSE | *ISA*: SSE2

  + [r64]    input = false  output = true
  + [xmm]    input = true   output = false
- *GAS*: movq | *GO*: MOVQ | *MMX*: MMX | *ISA*: MMX

  + [mm]     input = false  output = true
  + [r64]    input = true   output = false
- *GAS*: movq | *GO*: MOVQ | *MMX*: MMX | *ISA*: MMX

  + [mm]     input = false  output = true
  + [mm]     input = true   output = false
- *GAS*: movq | *GO*: MOVQ | *MMX*: MMX | *ISA*: MMX

  + [mm]     input = false  output = true
  + [m64]    input = true   output = false
- *GAS*: movq | *GO*: MOVQ | *XMM*: SSE | *ISA*: SSE2

  + [xmm]    input = false  output = true
  + [r64]    input = true   output = false
- *GAS*: movq | *GO*: MOVQ | *XMM*: SSE | *ISA*: SSE2

  + [xmm]    input = false  output = true
  + [xmm]    input = true   output = false
- *GAS*: movq | *GO*: MOVQ | *XMM*: SSE | *ISA*: SSE2

  + [xmm]    input = false  output = true
  + [m64]    input = true   output = false
- *GAS*: movq | *GO*: MOVQ | *MMX*: MMX | *ISA*: MMX

  + [m64]    input = false  output = true
  + [mm]     input = true   output = false
- *GAS*: movq | *GO*: MOVQ | *XMM*: SSE | *ISA*: SSE2

  + [m64]    input = false  output = true
  + [xmm]    input = true   output = false",
        );
    }

    #[test]
    fn handle_hover_x86_x86_64_it_provides_reg_info_normal() {
        test_hover(
            "	pushq	%r<cursor>bp",
            "RBP [x86]
Stack Base Pointer

Type: General Purpose Register
Width: 64 bits

RBP [x86-64]
Base Pointer (meant for stack frames)

Type: General Purpose Register
Width: 64 bits",
        );
    }
    #[test]
    fn handle_hover_x86_x86_64_it_provides_reg_info_offset() {
        test_hover(
            "	movl	%edi, -20(%r<cursor>bp)",
            "RBP [x86]
Stack Base Pointer

Type: General Purpose Register
Width: 64 bits

RBP [x86-64]
Base Pointer (meant for stack frames)

Type: General Purpose Register
Width: 64 bits",
        );
    }
    #[test]
    fn handle_hover_x86_x86_64_it_provies_reg_info_relative_addressing() {
        test_hover(
            "	leaq	_ZSt4cout(%<cursor>rip), %rdi",
            "RIP [x86]
Instruction Pointer

Type: Pointer Register
Width: 64 bits

RIP [x86-64]
Instruction Pointer. Can only be used in RIP-relative addressing.

Type: Pointer Register
Width: 64 bits",
        );
    }

    #[test]
    fn handle_hover_gas_it_provides_directive_info_1() {
        test_hover(r#"	.f<cursor>ile	"a.cpp"#, ".file [Gas]
This version of the `.file` directive tells `as` that we are about to start a new logical file. When emitting DWARF2 line number information, `.file` assigns filenames to the `.debug_line` file name table.

- .file *string*
- .file *fileno filename*

More info: https://sourceware.org/binutils/docs-2.41/as/File.html",
            );
    }
    #[test]
    fn handle_hover_gas_it_provides_directive_info_2() {
        test_hover(".<cursor>text", ".text [Gas]
Tells *as* to assemble the following statements onto the end of the text subsection numbered *subsection*, which is an absolute expression. If *subsection* is omitted, subsection number zero is used.

- .text *subsection*

More info: https://sourceware.org/binutils/docs-2.41/as/Text.html",
            );
    }
    #[test]
    fn handle_hover_gas_it_provides_directive_info_3() {
        test_hover("	.glob<cursor>l	main", ".globl [Gas]
`.globl` makes the symbol visible to `ld`. If you define symbol in your partial program, its value is made available to other partial programs that are linked with it.

- .globl *symbol*

More info: https://sourceware.org/binutils/docs-2.41/as/Global.html",
            );
    }

    #[test]
    fn handle_hover_it_demangles_cpp_1() {
        test_hover("	call	<cursor>_ZStlsISt11char_traitsIcEERSt13basic_ostreamIcT_ES5_PKc@PLT",
            "std::basic_ostream<char, std::char_traits<char> >& std::operator<< <std::char_traits<char> >(std::basic_ostream<char, std::char_traits<char> >&, char const*)",
            );
    }
    #[test]
    fn handle_hover_it_demangles_cpp_2() {
        test_hover("	leaq	_ZSt4c<cursor>out(%rip), %rdi", "std::cout");
    }
    #[test]
    fn handle_hover_it_demangles_cpp_3() {
        test_hover("	movq	_ZSt4endlIcSt<cursor>11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_@GOTPCREL(%rip), %rax",
        "std::basic_ostream<char, std::char_traits<char> >& std::endl<char, std::char_traits<char> >(std::basic_ostream<char, std::char_traits<char> >&)",
            );
    }

    #[test]
    fn handle_hover_z80_it_provides_instr_info_no_args() {
        test_hover("        LD<cursor>I             ;MOVE CHARACTER (HL) to (DE)",
"ldi [z80]
LoaD and Increment. Copies the byte pointed to by HL to the address pointed to by DE, then adds 1 to DE and HL and subtracts 1 from BC. P/V is set to (BC!=0), i.e. set when non zero.

## Forms

- *Z80*: LDI

  + Z80: 16, Z80 + M1: 18, R800: 4, R800 + Wait: 18
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LDI
",
            );
    }
    #[test]
    fn handle_hover_z80_it_provides_instr_info_one_reg_arg() {
        test_hover("        CP<cursor> (HL)         ;COMPARE MEMORY CONTENTS WITH",
            "cp [z80]
ComPare. Sets the flags as if a SUB was performed but does not perform it. Legal combinations are the same as SUB. This is commonly used to set the flags to perform an equality or greater/less test.

## Forms

- *Z80*: CP (HL)

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#CP%20%28HL%29

- *Z80*: CP (IX+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#CP%20%28IX%2Bo%29

- *Z80*: CP (IY+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#CP%20%28IY%2Bo%29

- *Z80*: CP n

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#CP%20n

- *Z80*: CP r

  + Z80: 4, Z80 + M1: 5, R800: 1, R800 + Wait: 5
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#CP%20r

- *Z80*: CP IXp

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#CP%20IXp

- *Z80*: CP IYq

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#CP%20IYq
",
            );
    }
    #[test]
    fn handle_hover_z80_it_provides_instr_info_two_reg_args() {
        test_hover("        L<cursor>D HL, DATA     ;STARTING ADDRESS OF DATA STRING",
"ld [z80]
LoaD. The basic data load/transfer instruction. Transfers data from the location specified by the second argument, to the location specified by the first.

## Forms

- *Z80*: LD (BC), A

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20%28BC%29%2C%20A

- *Z80*: LD (DE), A

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20%28DE%29%2C%20A

- *Z80*: LD (HL), n

  + Z80: 10, Z80 + M1: 11, R800: 3, R800 + Wait: 11
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20%28HL%29%2C%20n

- *Z80*: LD (HL), r

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20%28HL%29%2C%20r

- *Z80*: LD (IX+o), n

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20%28IX%2Bo%29%2C%20n

- *Z80*: LD (IX+o), r

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20%28IX%2Bo%29%2C%20r

- *Z80*: LD (IY+o), n

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20%28IY%2Bo%29%2C%20n

- *Z80*: LD (IY+o), r

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20%28IY%2Bo%29%2C%20r

- *Z80*: LD (nn), A

  + Z80: 13, Z80 + M1: 14, R800: 4, R800 + Wait: 14
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20%28nn%29%2C%20A

- *Z80*: LD (nn), BC

  + Z80: 20, Z80 + M1: 22, R800: 6, R800 + Wait: 22
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20%28nn%29%2C%20BC

- *Z80*: LD (nn), DE

  + Z80: 20, Z80 + M1: 22, R800: 6, R800 + Wait: 22
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20%28nn%29%2C%20DE

- *Z80*: LD (nn), HL

  + Z80: 16, Z80 + M1: 17, R800: 5, R800 + Wait: 17
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20%28nn%29%2C%20HL

- *Z80*: LD (nn), IX

  + Z80: 20, Z80 + M1: 22, R800: 6, R800 + Wait: 22
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20%28nn%29%2C%20IX

- *Z80*: LD (nn), IY

  + Z80: 20, Z80 + M1: 22, R800: 6, R800 + Wait: 22
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20%28nn%29%2C%20IY

- *Z80*: LD (nn), SP

  + Z80: 20, Z80 + M1: 22, R800: 6, R800 + Wait: 22
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20%28nn%29%2C%20SP

- *Z80*: LD A, (BC)

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20A%2C%20%28BC%29

- *Z80*: LD A, (DE)

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20A%2C%20%28DE%29

- *Z80*: LD A, (HL)

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20A%2C%20%28HL%29

- *Z80*: LD A, (IX+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20A%2C%20%28IX%2Bo%29

- *Z80*: LD A, (IY+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20A%2C%20%28IY%2Bo%29

- *Z80*: LD A, (nn)

  + Z80: 13, Z80 + M1: 14, R800: 4, R800 + Wait: 14
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20A%2C%20%28nn%29

- *Z80*: LD A, n

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20A%2C%20n

- *Z80*: LD A, r

  + Z80: 4, Z80 + M1: 5, R800: 1, R800 + Wait: 5
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20A%2C%20r

- *Z80*: LD A, IXp

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20A%2C%20IXp

- *Z80*: LD A, IYq

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20A%2C%20IYq

- *Z80*: LD A, I

  + Z80: 9, Z80 + M1: 11, R800: 2, R800 + Wait: 11
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20A%2C%20I

- *Z80*: LD A, R

  + Z80: 9, Z80 + M1: 11, R800: 2, R800 + Wait: 11
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20A%2C%20R

- *Z80*: LD B, (HL)

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20B%2C%20%28HL%29

- *Z80*: LD B, (IX+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20B%2C%20%28IX%2Bo%29

- *Z80*: LD B, (IY+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20B%2C%20%28IY%2Bo%29

- *Z80*: LD B, n

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20B%2C%20n

- *Z80*: LD B, r

  + Z80: 4, Z80 + M1: 5, R800: 1, R800 + Wait: 5
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20B%2C%20r

- *Z80*: LD B, IXp

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20B%2C%20IXp

- *Z80*: LD B, IYq

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20B%2C%20IYq

- *Z80*: LD BC, (nn)

  + Z80: 20, Z80 + M1: 22, R800: 6, R800 + Wait: 22
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20BC%2C%20%28nn%29

- *Z80*: LD BC, nn

  + Z80: 10, Z80 + M1: 11, R800: 3, R800 + Wait: 11
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20BC%2C%20nn

- *Z80*: LD C, (HL)

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20C%2C%20%28HL%29

- *Z80*: LD C, (IX+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20C%2C%20%28IX%2Bo%29

- *Z80*: LD C, (IY+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20C%2C%20%28IY%2Bo%29

- *Z80*: LD C, n

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20C%2C%20n

- *Z80*: LD C, r

  + Z80: 4, Z80 + M1: 5, R800: 1, R800 + Wait: 5
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20C%2C%20r

- *Z80*: LD C, IXp

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20C%2C%20IXp

- *Z80*: LD C, IYq

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20C%2C%20IYq

- *Z80*: LD D, (HL)

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20D%2C%20%28HL%29

- *Z80*: LD D, (IX+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20D%2C%20%28IX%2Bo%29

- *Z80*: LD D, (IY+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20D%2C%20%28IY%2Bo%29

- *Z80*: LD D, n

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20D%2C%20n

- *Z80*: LD D, r

  + Z80: 4, Z80 + M1: 5, R800: 1, R800 + Wait: 5
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20D%2C%20r

- *Z80*: LD D, IXp

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20D%2C%20IXp

- *Z80*: LD D, IYq

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20D%2C%20IYq

- *Z80*: LD DE, (nn)

  + Z80: 20, Z80 + M1: 22, R800: 6, R800 + Wait: 22
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20DE%2C%20%28nn%29

- *Z80*: LD DE, nn

  + Z80: 10, Z80 + M1: 11, R800: 3, R800 + Wait: 11
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20DE%2C%20nn

- *Z80*: LD E, (HL)

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20E%2C%20%28HL%29

- *Z80*: LD E, (IX+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20E%2C%20%28IX%2Bo%29

- *Z80*: LD E, (IY+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20E%2C%20%28IY%2Bo%29

- *Z80*: LD E, n

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20E%2C%20n

- *Z80*: LD E, r

  + Z80: 4, Z80 + M1: 5, R800: 1, R800 + Wait: 5
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20E%2C%20r

- *Z80*: LD E, IXp

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20E%2C%20IXp

- *Z80*: LD E, IYq

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20E%2C%20IYq

- *Z80*: LD H, (HL)

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20H%2C%20%28HL%29

- *Z80*: LD H, (IX+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20H%2C%20%28IX%2Bo%29

- *Z80*: LD H, (IY+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20H%2C%20%28IY%2Bo%29

- *Z80*: LD H, n

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20H%2C%20n

- *Z80*: LD H, r

  + Z80: 4, Z80 + M1: 5, R800: 1, R800 + Wait: 5
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20H%2C%20r

- *Z80*: LD HL, (nn)

  + Z80: 16, Z80 + M1: 17, R800: 5, R800 + Wait: 17
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20HL%2C%20%28nn%29

- *Z80*: LD HL, nn

  + Z80: 10, Z80 + M1: 11, R800: 3, R800 + Wait: 11
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20HL%2C%20nn

- *Z80*: LD I, A

  + Z80: 9, Z80 + M1: 11, R800: 2, R800 + Wait: 11
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20I%2C%20A

- *Z80*: LD IX, (nn)

  + Z80: 20, Z80 + M1: 22, R800: 6, R800 + Wait: 22
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20IX%2C%20%28nn%29

- *Z80*: LD IX, nn

  + Z80: 14, Z80 + M1: 16, R800: 4, R800 + Wait: 16
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20IX%2C%20nn

- *Z80*: LD IXh, n

  + Z80: 11, Z80 + M1: 13, R800: 3, R800 + Wait: 13
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20IXh%2C%20n

- *Z80*: LD IXh, p

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20IXh%2C%20p

- *Z80*: LD IXl, n

  + Z80: 11, Z80 + M1: 13, R800: 3, R800 + Wait: 13
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20IXl%2C%20n

- *Z80*: LD IXl, p

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20IXl%2C%20p

- *Z80*: LD IY, (nn)

  + Z80: 20, Z80 + M1: 22, R800: 6, R800 + Wait: 22
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20IY%2C%20%28nn%29

- *Z80*: LD IY, nn

  + Z80: 14, Z80 + M1: 16, R800: 4, R800 + Wait: 16
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20IY%2C%20nn

- *Z80*: LD IYh, n

  + Z80: 11, Z80 + M1: 13, R800: 3, R800 + Wait: 13
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20IYh%2C%20n

- *Z80*: LD IYh, q

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20IYh%2C%20q

- *Z80*: LD IYl, n

  + Z80: 11, Z80 + M1: 13, R800: 3, R800 + Wait: 13
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20IYl%2C%20n

- *Z80*: LD IYl, q

  + Z80: 8, Z80 + M1: 10, R800: 2, R800 + Wait: 10
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20IYl%2C%20q

- *Z80*: LD L, (HL)

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20L%2C%20%28HL%29

- *Z80*: LD L, (IX+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20L%2C%20%28IX%2Bo%29

- *Z80*: LD L, (IY+o)

  + Z80: 19, Z80 + M1: 21, R800: 5, R800 + Wait: 21
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20L%2C%20%28IY%2Bo%29

- *Z80*: LD L, n

  + Z80: 7, Z80 + M1: 8, R800: 2, R800 + Wait: 8
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20L%2C%20n

- *Z80*: LD L, r

  + Z80: 4, Z80 + M1: 5, R800: 1, R800 + Wait: 5
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20L%2C%20r

- *Z80*: LD R, A

  + Z80: 9, Z80 + M1: 11, R800: 2, R800 + Wait: 11
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20R%2C%20A

- *Z80*: LD SP, (nn)

  + Z80: 20, Z80 + M1: 22, R800: 6, R800 + Wait: 22
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20SP%2C%20%28nn%29

- *Z80*: LD SP, HL

  + Z80: 6, Z80 + M1: 7, R800: 1, R800 + Wait: 7
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20SP%2C%20HL

- *Z80*: LD SP, IX

  + Z80: 10, Z80 + M1: 12, R800: 2, R800 + Wait: 12
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20SP%2C%20IX

- *Z80*: LD SP, IY

  + Z80: 10, Z80 + M1: 12, R800: 2, R800 + Wait: 12
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20SP%2C%20IY

- *Z80*: LD SP, nn

  + Z80: 10, Z80 + M1: 11, R800: 3, R800 + Wait: 11
  + More info: https://www.zilog.com/docs/z80/z80cpu_um.pdf#LD%20SP%2C%20nn
"
            );
    }

    #[test]
    fn handle_hover_z80_it_provides_reg_info_normal() {
        test_hover(
            "        LD H<cursor>L, DATA     ;STARTING ADDRESS OF DATA STRING",
            "HL
16-bit accumulator/address register or two 8-bit registers.

Width: 16 bits",
        );
    }
    #[test]
    fn handle_hover_z80_it_provides_reg_info_prime() {
        test_hover(
            "        LD B<cursor>', 132      ;MAXIMUM STRING LENGTH",
            "B
General purpose register.

Type: General Purpose Register
Width: 8 bits",
        );
    }
}
