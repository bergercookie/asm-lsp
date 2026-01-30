#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, str::FromStr};

    use anyhow::Result;
    use lsp_textdocument::FullTextDocument;
    use lsp_types::{
        CompletionContext, CompletionItemKind, CompletionParams, CompletionTriggerKind,
        DidOpenTextDocumentParams, HoverContents, HoverParams, MarkupContent, MarkupKind,
        PartialResultParams, Position, TextDocumentIdentifier, TextDocumentItem,
        TextDocumentPositionParams, Uri, WorkDoneProgressParams,
    };
    use tree_sitter::Parser;

    use crate::{
        Arch, Assembler, BINCODE_CFG, Config, ConfigOptions, Directive, DocumentStore, Instruction,
        Register, ServerStore, TreeEntry, get_comp_resp, get_completes, get_hover_resp,
        get_word_from_pos_params, instr_filter_targets,
        parser::{
            populate_6502_instructions, populate_arm_instructions, populate_avr_directives,
            populate_avr_instructions, populate_ca65_directives, populate_mars_pseudo_instructions,
            populate_masm_nasm_fasm_mars_directives, populate_mips_instructions,
            populate_power_isa_instructions, populate_riscv_instructions, populate_riscv_registers,
        },
        populate_gas_directives, populate_instructions, populate_name_to_directive_map,
        populate_name_to_instruction_map, populate_name_to_register_map, populate_registers,
    };

    const fn empty_test_config() -> Config {
        Config {
            version: None,
            assembler: Assembler::None,
            instruction_set: Arch::None,
            opts: Some(ConfigOptions {
                compiler: None,
                compile_flags_txt: None,
                diagnostics: None,
                default_diagnostics: None,
            }),
        }
    }

    fn mips_arch_test_config() -> Config {
        Config {
            version: None,
            assembler: Assembler::None,
            instruction_set: Arch::Mips,
            opts: Some(ConfigOptions::default()),
        }
    }

    fn avr_arch_test_config() -> Config {
        Config {
            version: None,
            assembler: Assembler::None,
            instruction_set: Arch::Avr,
            opts: Some(ConfigOptions::default()),
        }
    }

    fn avr_assembler_test_config() -> Config {
        Config {
            version: None,
            assembler: Assembler::Avr,
            instruction_set: Arch::None,
            opts: Some(ConfigOptions::default()),
        }
    }

    fn power_isa_test_config() -> Config {
        Config {
            version: None,
            assembler: Assembler::None,
            instruction_set: Arch::PowerISA,
            opts: Some(ConfigOptions::default()),
        }
    }

    fn mos6502_test_config() -> Config {
        Config {
            version: None,
            assembler: Assembler::None,
            instruction_set: Arch::MOS6502,
            opts: Some(ConfigOptions::default()),
        }
    }

    fn z80_test_config() -> Config {
        Config {
            version: None,
            assembler: Assembler::None,
            instruction_set: Arch::Z80,
            opts: Some(ConfigOptions::default()),
        }
    }

    fn arm_test_config() -> Config {
        Config {
            version: None,
            assembler: Assembler::None,
            instruction_set: Arch::ARM,
            opts: Some(ConfigOptions::default()),
        }
    }

    fn riscv_test_config() -> Config {
        Config {
            version: None,
            assembler: Assembler::None,
            instruction_set: Arch::RISCV,
            opts: Some(ConfigOptions::default()),
        }
    }

    fn x86_x86_64_test_config() -> Config {
        Config {
            version: None,
            // HACK: The Gas or Go assembler must be enabled for this test
            // config, as filtering later on removes any x86/x86-64
            // instructions without any `form` fields that match one of
            // those assemblers. Currently, the tests are written with the
            // expectation of Gas being enbabled and Go disabled
            assembler: Assembler::Gas,
            instruction_set: Arch::X86_AND_X86_64,
            opts: Some(ConfigOptions::default()),
        }
    }

    fn gas_test_config() -> Config {
        Config {
            version: None,
            assembler: Assembler::Gas,
            instruction_set: Arch::None,
            opts: Some(ConfigOptions::default()),
        }
    }

    fn masm_test_config() -> Config {
        Config {
            version: None,
            assembler: Assembler::Masm,
            instruction_set: Arch::None,
            opts: Some(ConfigOptions::default()),
        }
    }

    fn nasm_test_config() -> Config {
        Config {
            version: None,
            assembler: Assembler::Nasm,
            instruction_set: Arch::None,
            opts: Some(ConfigOptions::default()),
        }
    }

    fn ca65_test_config() -> Config {
        Config {
            version: None,
            assembler: Assembler::Ca65,
            instruction_set: Arch::None,
            opts: Some(ConfigOptions::default()),
        }
    }

    fn fasm_test_config() -> Config {
        Config {
            version: None,
            assembler: Assembler::Fasm,
            instruction_set: Arch::None,
            opts: Some(ConfigOptions::default()),
        }
    }

    fn mars_test_config() -> Config {
        Config {
            version: None,
            assembler: Assembler::Mars,
            // NOTE: Mars only targets Mips, and we need the ISA enabled to access
            // Mars's pseudo instructions
            instruction_set: Arch::Mips,
            opts: Some(ConfigOptions::default()),
        }
    }

    #[derive(Debug)]
    struct GlobalInfo {
        x86_instructions: Vec<Instruction>,
        x86_64_instructions: Vec<Instruction>,
        x86_registers: Vec<Register>,
        x86_64_registers: Vec<Register>,
        arm_instructions: Vec<Instruction>,
        arm_registers: Vec<Register>,
        arm64_instructions: Vec<Instruction>,
        arm64_registers: Vec<Register>,
        riscv_instructions: Vec<Instruction>,
        mos6502_instructions: Vec<Instruction>,
        power_isa_instructions: Vec<Instruction>,
        riscv_registers: Vec<Register>,
        z80_instructions: Vec<Instruction>,
        z80_registers: Vec<Register>,
        mos6502_registers: Vec<Register>,
        power_isa_registers: Vec<Register>,
        avr_instructions: Vec<Instruction>,
        avr_registers: Vec<Register>,
        mips_instructions: Vec<Instruction>,
        mips_registers: Vec<Register>,
        gas_directives: Vec<Directive>,
        masm_directives: Vec<Directive>,
        nasm_directives: Vec<Directive>,
        ca65_directives: Vec<Directive>,
        avr_directives: Vec<Directive>,
        fasm_directives: Vec<Directive>,
        mars_directives: Vec<Directive>,
    }

    impl GlobalInfo {
        const fn new() -> Self {
            Self {
                x86_instructions: Vec::new(),
                x86_64_instructions: Vec::new(),
                x86_registers: Vec::new(),
                x86_64_registers: Vec::new(),
                arm_instructions: Vec::new(),
                arm_registers: Vec::new(),
                arm64_instructions: Vec::new(),
                arm64_registers: Vec::new(),
                riscv_instructions: Vec::new(),
                riscv_registers: Vec::new(),
                power_isa_instructions: Vec::new(),
                power_isa_registers: Vec::new(),
                z80_instructions: Vec::new(),
                z80_registers: Vec::new(),
                mos6502_instructions: Vec::new(),
                mos6502_registers: Vec::new(),
                avr_instructions: Vec::new(),
                avr_registers: Vec::new(),
                mips_instructions: Vec::new(),
                mips_registers: Vec::new(),
                gas_directives: Vec::new(),
                masm_directives: Vec::new(),
                nasm_directives: Vec::new(),
                ca65_directives: Vec::new(),
                avr_directives: Vec::new(),
                fasm_directives: Vec::new(),
                mars_directives: Vec::new(),
            }
        }
    }

    fn init_global_info(config: &Config) -> Result<GlobalInfo> {
        let mut info = GlobalInfo::new();

        info.x86_instructions = if config.is_isa_enabled(Arch::X86) {
            let x86_instrs = include_bytes!("serialized/opcodes/x86");
            bincode::borrow_decode_from_slice::<Vec<Instruction>, _>(x86_instrs, BINCODE_CFG)?
                .0
                .into_iter()
                .map(|instruction| {
                    // filter out assemblers by user config
                    instr_filter_targets(&instruction, config)
                })
                .filter(|instruction| !instruction.forms.is_empty())
                .collect()
        } else {
            Vec::new()
        };

        info.x86_64_instructions = if config.is_isa_enabled(Arch::X86_64) {
            let x86_64_instrs = include_bytes!("serialized/opcodes/x86_64");
            bincode::borrow_decode_from_slice::<Vec<Instruction>, _>(x86_64_instrs, BINCODE_CFG)?
                .0
                .into_iter()
                .map(|instruction| {
                    // filter out assemblers by user config
                    instr_filter_targets(&instruction, config)
                })
                .filter(|instruction| !instruction.forms.is_empty())
                .collect()
        } else {
            Vec::new()
        };

        info.z80_instructions = if config.is_isa_enabled(Arch::Z80) {
            let z80_instrs = include_bytes!("serialized/opcodes/z80");
            bincode::borrow_decode_from_slice::<Vec<Instruction>, _>(z80_instrs, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.arm_instructions = if config.is_isa_enabled(Arch::ARM) {
            let arm_instrs = include_bytes!("serialized/opcodes/arm");
            bincode::borrow_decode_from_slice::<Vec<Instruction>, _>(arm_instrs, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.arm64_instructions = if config.is_isa_enabled(Arch::ARM64) {
            let arm64_instrs = include_bytes!("serialized/opcodes/arm64");
            bincode::borrow_decode_from_slice::<Vec<Instruction>, _>(arm64_instrs, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.riscv_instructions = if config.is_isa_enabled(Arch::RISCV) {
            let riscv_instrs = include_bytes!("serialized/opcodes/riscv");
            bincode::borrow_decode_from_slice::<Vec<Instruction>, _>(riscv_instrs, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.mos6502_instructions = if config.is_isa_enabled(Arch::MOS6502) {
            let mos6502_instrs = include_bytes!("serialized/opcodes/6502");
            bincode::borrow_decode_from_slice::<Vec<Instruction>, _>(mos6502_instrs, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.power_isa_instructions = if config.is_isa_enabled(Arch::PowerISA) {
            let power_isa_instrs = include_bytes!("serialized/opcodes/power-isa");
            bincode::borrow_decode_from_slice::<Vec<Instruction>, _>(power_isa_instrs, BINCODE_CFG)?
                .0
        } else {
            Vec::new()
        };

        info.avr_instructions = if config.is_isa_enabled(Arch::Avr) {
            let avr_instrs = include_bytes!("serialized/opcodes/avr");
            bincode::borrow_decode_from_slice::<Vec<Instruction>, _>(avr_instrs, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.mips_instructions = if config.is_isa_enabled(Arch::Mips) {
            let mips_instrs = include_bytes!("serialized/opcodes/mips");
            bincode::borrow_decode_from_slice::<Vec<Instruction>, _>(mips_instrs, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        if config.is_assembler_enabled(Assembler::Mars) {
            let mars_pseudo_instrs = include_bytes!("serialized/opcodes/mars");
            info.mips_instructions
                .append(&mut bincode::borrow_decode_from_slice(mars_pseudo_instrs, BINCODE_CFG)?.0);
        }

        info.x86_registers = if config.is_isa_enabled(Arch::X86) {
            let regs_x86 = include_bytes!("serialized/registers/x86");
            bincode::borrow_decode_from_slice(regs_x86, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.x86_64_registers = if config.is_isa_enabled(Arch::X86_64) {
            let regs_x86_64 = include_bytes!("serialized/registers/x86_64");
            bincode::borrow_decode_from_slice(regs_x86_64, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.z80_registers = if config.is_isa_enabled(Arch::Z80) {
            let regs_z80 = include_bytes!("serialized/registers/z80");
            bincode::borrow_decode_from_slice(regs_z80, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.arm_registers = if config.is_isa_enabled(Arch::ARM) {
            let regs_arm = include_bytes!("serialized/registers/arm");
            bincode::borrow_decode_from_slice(regs_arm, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.arm64_registers = if config.is_isa_enabled(Arch::ARM64) {
            let regs_arm64 = include_bytes!("serialized/registers/arm64");
            bincode::borrow_decode_from_slice(regs_arm64, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.riscv_registers = if config.is_isa_enabled(Arch::RISCV) {
            let regs_riscv = include_bytes!("serialized/registers/riscv");
            bincode::borrow_decode_from_slice(regs_riscv, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.mos6502_registers = if config.is_isa_enabled(Arch::MOS6502) {
            let regs_mos6502 = include_bytes!("serialized/registers/6502");
            bincode::borrow_decode_from_slice(regs_mos6502, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.power_isa_registers = if config.is_isa_enabled(Arch::PowerISA) {
            let regs_power_isa = include_bytes!("serialized/registers/power-isa");
            bincode::borrow_decode_from_slice(regs_power_isa, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.avr_registers = if config.is_isa_enabled(Arch::Avr) {
            let regs_avr = include_bytes!("serialized/registers/avr");
            bincode::borrow_decode_from_slice(regs_avr, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.mips_registers = if config.is_isa_enabled(Arch::Mips) {
            let regs_mips = include_bytes!("serialized/registers/mips");
            bincode::borrow_decode_from_slice(regs_mips, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.gas_directives = if config.is_assembler_enabled(Assembler::Gas) {
            let gas_dirs = include_bytes!("serialized/directives/gas");
            bincode::borrow_decode_from_slice(gas_dirs, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.masm_directives = if config.is_assembler_enabled(Assembler::Masm) {
            let masm_dirs = include_bytes!("serialized/directives/masm");
            bincode::borrow_decode_from_slice(masm_dirs, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.nasm_directives = if config.is_assembler_enabled(Assembler::Nasm) {
            let nasm_dirs = include_bytes!("serialized/directives/nasm");
            bincode::borrow_decode_from_slice(nasm_dirs, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.ca65_directives = if config.is_assembler_enabled(Assembler::Ca65) {
            let ca65_dirs = include_bytes!("serialized/directives/ca65");
            bincode::borrow_decode_from_slice(ca65_dirs, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.avr_directives = if config.is_assembler_enabled(Assembler::Avr) {
            let avr_dirs = include_bytes!("serialized/directives/avr");
            bincode::borrow_decode_from_slice(avr_dirs, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.fasm_directives = if config.is_assembler_enabled(Assembler::Fasm) {
            let fasm_dirs = include_bytes!("serialized/directives/fasm");
            bincode::borrow_decode_from_slice(fasm_dirs, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        info.mars_directives = if config.is_assembler_enabled(Assembler::Mars) {
            let fasm_dirs = include_bytes!("serialized/directives/mars");
            bincode::borrow_decode_from_slice(fasm_dirs, BINCODE_CFG)?.0
        } else {
            Vec::new()
        };

        Ok(info)
    }

    fn init_test_store(info: &GlobalInfo) -> ServerStore {
        let mut store = ServerStore::default();

        populate_name_to_instruction_map(
            Arch::X86,
            &info.x86_instructions,
            &mut store.names_to_info.instructions,
        );

        populate_name_to_instruction_map(
            Arch::X86_64,
            &info.x86_64_instructions,
            &mut store.names_to_info.instructions,
        );

        populate_name_to_instruction_map(
            Arch::ARM,
            &info.arm_instructions,
            &mut store.names_to_info.instructions,
        );

        populate_name_to_instruction_map(
            Arch::ARM64,
            &info.arm64_instructions,
            &mut store.names_to_info.instructions,
        );

        populate_name_to_instruction_map(
            Arch::RISCV,
            &info.riscv_instructions,
            &mut store.names_to_info.instructions,
        );

        populate_name_to_instruction_map(
            Arch::Z80,
            &info.z80_instructions,
            &mut store.names_to_info.instructions,
        );

        populate_name_to_instruction_map(
            Arch::MOS6502,
            &info.mos6502_instructions,
            &mut store.names_to_info.instructions,
        );

        populate_name_to_instruction_map(
            Arch::PowerISA,
            &info.power_isa_instructions,
            &mut store.names_to_info.instructions,
        );

        populate_name_to_instruction_map(
            Arch::Avr,
            &info.avr_instructions,
            &mut store.names_to_info.instructions,
        );

        populate_name_to_instruction_map(
            Arch::Mips,
            &info.mips_instructions,
            &mut store.names_to_info.instructions,
        );

        populate_name_to_register_map(
            Arch::X86,
            &info.x86_registers,
            &mut store.names_to_info.registers,
        );

        populate_name_to_register_map(
            Arch::X86_64,
            &info.x86_64_registers,
            &mut store.names_to_info.registers,
        );

        populate_name_to_register_map(
            Arch::ARM,
            &info.arm_registers,
            &mut store.names_to_info.registers,
        );

        populate_name_to_register_map(
            Arch::ARM64,
            &info.arm64_registers,
            &mut store.names_to_info.registers,
        );

        populate_name_to_register_map(
            Arch::RISCV,
            &info.riscv_registers,
            &mut store.names_to_info.registers,
        );

        populate_name_to_register_map(
            Arch::MOS6502,
            &info.mos6502_registers,
            &mut store.names_to_info.registers,
        );

        populate_name_to_register_map(
            Arch::Z80,
            &info.z80_registers,
            &mut store.names_to_info.registers,
        );

        populate_name_to_register_map(
            Arch::PowerISA,
            &info.power_isa_registers,
            &mut store.names_to_info.registers,
        );

        populate_name_to_register_map(
            Arch::Avr,
            &info.avr_registers,
            &mut store.names_to_info.registers,
        );

        populate_name_to_register_map(
            Arch::Mips,
            &info.mips_registers,
            &mut store.names_to_info.registers,
        );

        populate_name_to_directive_map(
            Assembler::Gas,
            &info.gas_directives,
            &mut store.names_to_info.directives,
        );

        populate_name_to_directive_map(
            Assembler::Masm,
            &info.masm_directives,
            &mut store.names_to_info.directives,
        );

        populate_name_to_directive_map(
            Assembler::Nasm,
            &info.nasm_directives,
            &mut store.names_to_info.directives,
        );

        populate_name_to_directive_map(
            Assembler::Ca65,
            &info.ca65_directives,
            &mut store.names_to_info.directives,
        );

        populate_name_to_directive_map(
            Assembler::Avr,
            &info.avr_directives,
            &mut store.names_to_info.directives,
        );

        populate_name_to_directive_map(
            Assembler::Fasm,
            &info.fasm_directives,
            &mut store.names_to_info.directives,
        );

        populate_name_to_directive_map(
            Assembler::Mars,
            &info.mars_directives,
            &mut store.names_to_info.directives,
        );

        store.completion_items.instructions = get_completes(
            &store.names_to_info.instructions,
            Some(CompletionItemKind::OPERATOR),
        );

        store.completion_items.registers = get_completes(
            &store.names_to_info.registers,
            Some(CompletionItemKind::VARIABLE),
        );

        store.completion_items.directives = get_completes(
            &store.names_to_info.directives,
            Some(CompletionItemKind::OPERATOR),
        );

        store
    }

    fn test_hover(source: &str, expected: &str, config: &Config) {
        let info = init_global_info(config).expect("Failed to load info");
        let store = init_test_store(&info);

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

        let source_code = source.replace("<cursor>", "");
        let curr_doc = Some(FullTextDocument::new(
            "asm".to_string(),
            0,
            source_code.clone(),
        ));
        let uri: Uri = Uri::from_str("file://").unwrap();

        let mut doc_store = DocumentStore::new();
        // mock the didOpen notification to insert a new document
        let method = "textDocument/didOpen";
        let did_open_params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "asm".to_string(),
                version: 0,
                text: source_code.clone(),
            },
        };
        let params = serde_json::to_value(did_open_params).unwrap();
        doc_store.text_store.listen(method, &params);

        let pos_params = TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: position.expect("No <cursor> marker found"),
        };

        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_asm::language()).unwrap();
        let tree = parser.parse(&source_code, None);
        let tree_entry = TreeEntry { tree, parser };
        doc_store.tree_store.insert(uri, tree_entry);

        let hover_params = HoverParams {
            text_document_position_params: pos_params.clone(),
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
        };

        let (word, cursor_offset) = curr_doc.as_ref().map_or_else(
            || {
                panic!("No document");
            },
            |doc| get_word_from_pos_params(doc, &pos_params),
        );

        let resp = get_hover_resp(
            &hover_params,
            config,
            word,
            cursor_offset,
            &mut doc_store,
            &store,
        )
        .expect("Received empty hover response");

        if let HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: resp_text,
        }) = resp.contents
        {
            let cleaned = resp_text
                .replace("\n\n\n\n", "\n\n") // HACK:: not sure what's going on here...
                .replace("\n\n\n", "\n\n"); // ...or here
            assert_eq!(expected, cleaned);
        } else {
            panic!("Invalid hover response contents: {:?}", resp.contents);
        }
    }

    fn test_autocomplete(
        source: &str,
        config: &Config,
        expected_kind: CompletionItemKind,
        trigger_kind: CompletionTriggerKind,
        trigger_character: Option<String>,
    ) {
        let info = init_global_info(config).expect("Failed to load info");
        let globals = init_test_store(&info);

        let source_code = source.replace("<cursor>", "");

        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_asm::language()).unwrap();
        let tree = parser.parse(&source_code, None);
        let mut tree_entry = TreeEntry { tree, parser };

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
                uri: Uri::from_str("file://").unwrap(),
            },
            position: position.expect("No <cursor> marker found"),
        };

        let comp_ctx = CompletionContext {
            trigger_kind,
            trigger_character,
        };

        let params = CompletionParams {
            text_document_position: pos_params,
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
            context: Some(comp_ctx),
        };

        let resp = get_comp_resp(
            &source_code,
            &mut tree_entry,
            &params,
            config,
            &globals.completion_items,
        )
        .expect("Received empty completion response");

        // - We currently have a very course-grained approach to completions,
        // - We just send all of the appropriate items (e.g. all instructions, all
        // registers, or all directives) and let the editor's lsp client sort out
        // which to display/ in what order
        // - Given this, we won't check for equality for all of the expected items,
        // but instead just that
        //      1) There are some items
        //      2) Said items are of the right type
        // NOTE: Both instructions and directives use the OPERATOR complection type,
        // so another means of verification should be added here
        assert!(!resp.items.is_empty());
        for comp in &resp.items {
            assert!(comp.kind == Some(expected_kind));
        }
    }

    fn test_register_autocomplete(
        source: &str,
        config: &Config,
        trigger_kind: CompletionTriggerKind,
        trigger_character: Option<String>,
    ) {
        let expected_kind = CompletionItemKind::VARIABLE;
        test_autocomplete(
            source,
            config,
            expected_kind,
            trigger_kind,
            trigger_character,
        );
    }

    fn test_instruction_autocomplete(
        source: &str,
        config: &Config,
        trigger_kind: CompletionTriggerKind,
        trigger_character: Option<String>,
    ) {
        let expected_kind = CompletionItemKind::OPERATOR;
        test_autocomplete(
            source,
            config,
            expected_kind,
            trigger_kind,
            trigger_character,
        );
    }

    fn test_directive_autocomplete(
        source: &str,
        config: &Config,
        trigger_kind: CompletionTriggerKind,
        trigger_character: Option<String>,
    ) {
        let expected_kind = CompletionItemKind::OPERATOR;
        test_autocomplete(
            source,
            config,
            expected_kind,
            trigger_kind,
            trigger_character,
        );
    }

    fn test_label_autocomplete(
        source: &str,
        trigger_kind: CompletionTriggerKind,
        trigger_character: Option<String>,
    ) {
        let expected_kind = CompletionItemKind::VARIABLE;
        test_autocomplete(
            source,
            &empty_test_config(),
            expected_kind,
            trigger_kind,
            trigger_character,
        );
    }

    /**************************************************************************
     * Mips Tests
     *************************************************************************/
    #[test]
    fn handle_hover_mips_arch_it_provides_reg_info_1() {
        test_hover(
            "li $<cursor>v0, 4 # system call code for printing string = 4",
            "$V0 [mips]
The $v Registers are used for returning values from functions. They are not preserved across function calls. Aliased to $2

Type: Special Purpose Register",
            &mips_arch_test_config(),
        );
    }
    #[test]
    fn handle_hover_mips_arch_it_provides_reg_info_2() {
        test_hover(
            "la <cursor>$4, out_string # load address of string to be printed into $a0",
            "$4 [mips]
The $a registers are used for passing arguments to functions. They are not preserved across function calls. Aliased to $a0

Type: Special Purpose Register",
            &mips_arch_test_config(),
        );
    }
    #[test]
    fn handle_hover_mips_arch_it_provides_instr_info_1() {
        test_hover(
            "sysca<cursor>ll # call operating system to perform operation",
            "syscall [mips]
To cause a System Call exception
A system call exception occurs, immediately and unconditionally transferring control to the exception handler.
The code field is available for use as software parameters, but is retrieved by the exception handler only by loading the contents of the memory word containing the instruction.

## Templates

 + `SYSCALL `

More info: https://www.cs.cornell.edu/courses/cs3410/2008fa/MIPS_Vol2.pdf",
            &mips_arch_test_config(),
        );
    }
    #[test]
    fn handle_autocomplete_mips_arch_it_provides_reg_comps_1() {
        test_register_autocomplete(
            "li $<cursor>",
            &mips_arch_test_config(),
            CompletionTriggerKind::TRIGGER_CHARACTER,
            Some("$".to_string()),
        );
    }
    #[test]
    fn handle_autocomplete_mips_arch_it_provides_reg_comps_2() {
        test_register_autocomplete(
            "li $1<cursor>",
            &mips_arch_test_config(),
            CompletionTriggerKind::TRIGGER_CHARACTER,
            Some("$".to_string()),
        );
    }
    #[test]
    fn handle_autocomplete_mips_arch_it_provides_instr_comps_1() {
        test_instruction_autocomplete(
            "sys<cursor>",
            &mips_arch_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_mips_arch_it_provides_instr_comps_2() {
        test_instruction_autocomplete(
            "add<cursor>",
            &mips_arch_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    /**************************************************************************
     * Mars Tests
     *************************************************************************/
    #[test]
    fn handle_hover_mars_it_provides_dir_info_1() {
        test_hover(
            ".mac<cursor>ro foo",
            ".macro [mars]
A pattern-matching and replacement facility that provides a simple mechanism to name a frequently used sequence of instructions",
            &mars_test_config()
        );
    }
    #[test]
    fn handle_hover_mars_it_provides_dir_info_2() {
        test_hover(
            ".eq<cursor>v FOO 1",
            ".eqv [mars]
The .eqv directive (short for \"equivalence\") is similar to #define in C or C++. It is used to substitute an arbitrary string for an identifier. Using .eqv, you can specify simple substitutions that provide \"define once, use many times\" capability at assembly pre-processing time.",
            &mars_test_config()
        );
    }
    #[test]
    fn handle_hover_mars_it_provides_dir_info_3() {
        test_hover(
            ".inclu<cursor>de \"foo.s\"",
            ".include [mars]
The .include directive has one operand, a quoted filename. When the directive is carried out, the contents of the specified file are substituted for the directive. This occurs during assembly preprocessing. It is like #include in C or C++.",
            &mars_test_config()
        );
    }
    #[test]
    fn handle_autocomplete_mars_it_provides_dir_comps_1() {
        test_directive_autocomplete(
            ".m<cursor>",
            &mars_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_mars_it_provides_dir_comps_2() {
        test_directive_autocomplete(
            ".<cursor>",
            &mars_test_config(),
            CompletionTriggerKind::TRIGGER_CHARACTER,
            Some(".".to_string()),
        );
    }
    #[test]
    fn handle_autocomplete_mars_it_provides_pseudo_op_comps_1() {
        test_directive_autocomplete(
            "l<cursor>",
            &mars_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_mars_it_provides_pseudo_op_comps_2() {
        test_directive_autocomplete(
            "ad<cursor>",
            &mars_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    /**************************************************************************
     * Fasm Tests
     *************************************************************************/
    #[test]
    fn handle_hover_fasm_it_provides_dir_info_1() {
        test_hover(
            "for<cursor>mat ELF64 executable 3",
            "format [fasm]
The format directive followed by the format identifier allows to select the output format. This directive should be put at the beginning of the source. It can always be followed in the same line by the as keyword and the quoted string specifying the default file extension for the output file. Unless the output file name was specified from the command line, assembler will use this extension when generating the output file.",
            &fasm_test_config()
        );
    }
    #[test]
    fn handle_hover_fasm_it_provides_dir_info_2() {
        test_hover(
            "segm<cursor>ent readable executable",
            "segment [fasm]
The segment directive defines a new segment, it should be followed by label, which value will be the number of defined segment, optionally use16 or use32 word can follow to specify whether code in this segment should be 16-bit or 32-bit. The origin of segment is aligned to paragraph (16 bytes). All the labels defined then will have values relative to the beginning of this segment.",
            &fasm_test_config()
        );
    }
    #[test]
    fn handle_hover_fasm_it_provides_dir_info_3() {
        test_hover(
            "segm<ent readable execu<cursor>table",
            "executable [fasm]
Flag for the section directive.",
            &fasm_test_config(),
        );
    }
    #[test]
    fn handle_autocomplete_fasm_it_provides_dir_comps_1() {
        test_directive_autocomplete(
            "for<cursor>",
            &fasm_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_fasm_it_provides_dir_comps_2() {
        test_directive_autocomplete(
            "ent<cursor>",
            &fasm_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_fasm_it_provides_dir_comps_3() {
        test_directive_autocomplete(
            "seg<cursor>",
            &fasm_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    /**************************************************************************
     * AVR Tests
     *************************************************************************/
    #[test]
    fn handle_hover_avr_arch_it_provides_reg_info_1() {
        test_hover(
            "ldi     r1<cursor>6,0xC0",
            "R16 [AVR]
General purpose register 16. Mapped to address 0x10.

Type: General Purpose Register
Width: 8 bits",
            &avr_arch_test_config(),
        );
    }
    #[test]
    fn handle_hover_avr_arch_it_provides_reg_info_2() {
        test_hover(
            "clr r<cursor>17",
            "R17 [AVR]
General purpose register 17. Mapped to address 0x11.

Type: General Purpose Register
Width: 8 bits",
            &avr_arch_test_config(),
        );
    }
    #[test]
    fn handle_hover_avr_arch_it_provides_reg_info_3() {
        test_hover(
            "x<cursor>",
            "X [AVR]
General purpose X-register. Low byte is r26 and high byte is r27.

Type: General Purpose Register
Width: 16 bits",
            &avr_arch_test_config(),
        );
    }
    #[test]
    fn handle_hover_avr_arch_it_provides_instr_info_1() {
        test_hover(
            "ld<cursor>i     r16,0xC0",
            "ldi [AVR]

## Forms

- *AVR*: LDI (All)

  + [Rd]
  + [K]

Load Immediate

I T H S V N Z C
- - - - - - - -

Timing: AVRE: 1 | AVRXM: 1 | AVRXT: 1 | AVRRC: 1
",
            &avr_arch_test_config(),
        );
    }
    #[test]
    fn handle_hover_avr_arch_it_provides_instr_info_2() {
        test_hover(
            "c<cursor>lr r17",
            "clr [AVR]

## Forms

- *AVR*: CLR (All)

  + [Rd]

Clear Register

I T H S V N Z C
- - - 0 0 0 - 1

Timing: AVRE: 1 | AVRXM: 1 | AVRXT: 1 | AVRRC: 1
",
            &avr_arch_test_config(),
        );
    }
    #[test]
    fn handle_hover_avr_arch_it_provides_instr_info_3() {
        test_hover(
            "   ou<cursor>t 0x25,r16",
            "out [AVR]

## Forms

- *AVR*: OUT (All)

  + [A]
  + [Rr]

Out To I/O Location

I T H S V N Z C
- - - - - - - -

Timing: AVRE: 1 | AVRXM: 1 | AVRXT: 1 | AVRRC: 1
",
            &avr_arch_test_config(),
        );
    }
    #[test]
    fn handle_autocomplete_avr_arch_it_provides_reg_comps_1() {
        test_register_autocomplete(
            "ldi	r<cursor>",
            &avr_arch_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_avr_arch_it_provides_reg_comps_2() {
        test_register_autocomplete(
            "ldi	r1<cursor>,0xC0",
            &avr_arch_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_avr_arch_it_provides_reg_comps_3() {
        test_register_autocomplete(
            "fmuls	r16,r1<cursor>",
            &avr_arch_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_avr_arch_it_provides_instr_comps_1() {
        test_instruction_autocomplete(
            "l<cursor>	r1",
            &avr_arch_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_avr_arch_it_provides_instr_comps_2() {
        test_instruction_autocomplete(
            "ld<cursor>	r1,0xC0",
            &avr_arch_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_avr_arch_it_provides_instr_comps_3() {
        test_instruction_autocomplete(
            "f<cursor>	r16,r1",
            &avr_arch_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_avr_assembler_it_provides_dir_comps_no_args() {
        test_directive_autocomplete(
            ".und<cursor>",
            &avr_assembler_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_avr_assembler_it_provides_dir_comps_args_1() {
        test_directive_autocomplete(
            ".or<cursor> 0x120 ; Set SRAM address to hex 120",
            &avr_assembler_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_avr_assembler_it_provides_dir_comps_args_2() {
        test_directive_autocomplete(
            ".incl<cursor> <foo>",
            &avr_assembler_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_hover_avr_assembler_it_provides_dir_info_1() {
        test_hover(
            ".incl<cursor>ude <foo>",
            r#".include [avr]
Include another file.
The INCLUDE directive tells the Assembler to start reading from a specified file. The Assembler then
assembles the specified file until end of file (EOF) or an EXIT directive is encountered. An included file
may contain INCLUDE directives itself. The difference between the two forms is that the first one search
the current directory first, the second one does not.

- .INCLUDE "filename"
- .INCLUDE <filename>"#,
            &avr_assembler_test_config(),
        );
    }

    #[test]
    fn handle_hover_avr_assembler_it_provides_dir_info_2() {
        test_hover(
            ".or<cursor>g 0x0000",
            ".org [avr]
Set program origin.
The ORG directive sets the location counter to an absolute value. The value to set is given as a
parameter. If an ORG directive is given within a Data Segment, then it is the SRAM location counter
which is set, if the directive is given within a Code Segment, then it is the Program memory counter which
is set and if the directive is given within an EEPROM Segment, it is the EEPROM location counter which
is set.
The default values of the Code and the EEPROM location counters are zero, and the default value of the
SRAM location counter is the address immediately following the end of I/O address space (0x60 for
devices without extended I/O, 0x100 or more for devices with extended I/O) when the assembling is
started. Note that the SRAM and EEPROM location counters count bytes whereas the Program memory
location counter counts words. Also note that some devices lack SRAM and/or EEPROM.

- .ORG expression",
            &avr_assembler_test_config(),
        );
    }

    /**************************************************************************
     * PowerISA Tests
     *************************************************************************/
    #[test]
    fn handle_autocomplete_power_isa_it_provides_instr_comps_no_args() {
        test_instruction_autocomplete(
            "lb<cursor>",
            &power_isa_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_power_isa_it_provides_instr_comps_existing_args_1() {
        test_instruction_autocomplete(
            "l<cursor> r4, 0(r3)",
            &power_isa_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_power_isa_it_provides_instr_comps_existing_args_2() {
        test_instruction_autocomplete(
            "add<cursor> r3, r3, 1",
            &power_isa_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_power_isa_it_provides_reg_comps_1() {
        test_register_autocomplete(
            "lbz r<cursor>",
            &power_isa_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_power_isa_it_provides_reg_comps_2() {
        test_register_autocomplete(
            "lbz r4, 0(r<cursor>)",
            &power_isa_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_power_isa_it_provides_reg_comps_existing_args_3() {
        test_register_autocomplete(
            "addi r<cursor>, r3, 1",
            &power_isa_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_hover_power_isa_it_provides_instr_info_1() {
        test_hover(
            "lb<cursor>z r4, 0(r3)",
            "lbz [power-isa]

Load Byte and Zero (Introduced in POWER Architecture)

 + `D(RA)`

For lbz, let the effective address (EA) be the sum (RA|0)+EXTS64(D).

For plbz with R=0, let EA
be the sum of the contents of register RA, or the value 0
if RA=0, and the value d0||d1,
sign-extended to 64 bits.

For plbz with R=1, let EA
be the sum of the address of the instruction and the value d0||d1, sign-extended to 64 bits.

The byte in storage addressed by EA is loaded into
RT56:63.
RT0:55 are set to 0.

For plbz, if R is equal to 1 and RA is not equal to 0, the instruction form is
invalid.

",
            &power_isa_test_config(),
        );
    }

    #[test]
    fn handle_hover_power_isa_it_provides_instr_info_2() {
        test_hover(
            "        add<cursor>i r4, r4, 0x20   # 'a' - 'A'",
            "addi [power-isa]

Add Immediate (Introduced in POWER Architecture)

 + `SI`

For addi, let the sum of the contents of register
RA, or the value 0 if RA=0, and the value
SI, sign-extended to 64 bits, is placed into register
RT.

For paddi with R=0, the sum of the
contents of register RA, or the value 0 if RA=0, and the value si0||si1, sign-extended
to 64 bits, is placed into register RT.

For paddi with R=1, the sum of the
address of the instruction and the value si0||si1, sign-extended to 64 bits, is placed into register RT.

For paddi, if R is equal to
1 and RA is not equal to 0, the instruction
form is invalid.

",
            &power_isa_test_config(),
        );
    }

    #[test]
    fn handle_hover_power_isa_it_provides_instr_info_3() {
        test_hover(
            "b<cursor> loop",
            "b [power-isa]

Branch (Introduced in POWER Architecture)

 + `target_addr`

target_addr specifies the branch target address.

If AA=0 then the branch target address is the sum of LI||0b00
sign-extended and the address of this instruction, with the
high-order 32 bits of the branch target address set to 0 in 32-bit
mode.

If AA=1 then the branch target address is the value LI||0b00
sign-extended, with the high-order 32 bits of the branch target
address set to 0 in 32-bit mode.

If LK=1 then the effective address of the instruction following the
Branch instruction is placed into the Link Register.

",
            &power_isa_test_config(),
        );
    }

    #[test]
    fn handle_hover_power_isa_it_provides_reg_info_1() {
        test_hover(
            "lbz r<cursor>4, 0(r3)",
            "R4 [power-isa]
For integer operations. 32 bit width prior to PowerPC Architecture Version 2.01.

Type: General Purpose Register
Width: 64 bits",
            &power_isa_test_config(),
        );
    }

    #[test]
    fn handle_hover_power_isa_it_provides_reg_info_2() {
        test_hover(
            "lbz r4, 0(r<cursor>3)",
            "R3 [power-isa]
For integer operations. 32 bit width prior to PowerPC Architecture Version 2.01.

Type: General Purpose Register
Width: 64 bits",
            &power_isa_test_config(),
        );
    }

    #[test]
    fn handle_hover_power_isa_it_provides_reg_info_3() {
        test_hover(
            "fadd f<cursor>3, f3, 1.0",
            "F3 [power-isa]
For floating point operations.

Type: Floating Point Register
Width: 64 bits",
            &power_isa_test_config(),
        );
    }

    /**************************************************************************
     * Ca65 Tests
     *************************************************************************/
    #[test]
    fn handle_autocomplete_ca65_it_provides_comps_1() {
        test_directive_autocomplete(
            ".en<cursor>",
            &ca65_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_ca65_it_provides_comps_2() {
        test_directive_autocomplete(
            r#".ou<cursor> "An out message""#,
            &ca65_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_ca65_it_provides_comps_3() {
        test_directive_autocomplete(
            ".<cursor>",
            &ca65_test_config(),
            CompletionTriggerKind::TRIGGER_CHARACTER,
            Some('.'.to_string()),
        );
    }

    #[test]
    fn handle_hover_ca65_it_provides_info_asterisk() {
        test_hover("*<cursor>", "* [ca65]
Reading this pseudo variable will return the program counter at the start of the current input line. Assignment to this variable is possible when .FEATURE pc_assignment is used. Note: You should not use assignments to *, use .ORG instead.

More info: https://cc65.github.io/doc/ca65.html#ss9.1",&ca65_test_config());
    }

    #[test]
    fn handle_hover_ca65_it_provides_info_1() {
        test_hover(".end<cursor>mac", ".endmac [ca65]
Marks the end of a macro definition. Note, .ENDMACRO should be on its own line to successfully end the macro definition. It is possible to use .DEFINE to create a symbol that references .ENDMACRO without ending the macro definition. Example:     .macro new_mac         .define startmac .macro         .define endmac .endmacro     .endmacro  See: .DELMACRO .EXITMACRO .MACRO See also section Macros.

More info: https://cc65.github.io/doc/ca65.html#ss11.28", &ca65_test_config());
    }

    #[test]
    fn handle_hover_ca65_it_provides_info_2() {
        test_hover(
            "<cursor>.byte $08, $0d",
            r#".byte [ca65]
Define byte sized data. Must be followed by a sequence of (byte ranged) expressions or strings. Strings will be translated using the current character mapping definition. Example:     .byte  "Hello "     .byt  "world", $0D, $00  See: .ASCIIZ, .CHARMAP .LITERAL

More info: https://cc65.github.io/doc/ca65.html#ss11.10"#,
            &ca65_test_config(),
        );
    }

    /**************************************************************************
     * 6502 Tests
     *************************************************************************/
    #[test]
    fn handle_autocomplete_6502_it_provides_instr_comps_no_args() {
        test_instruction_autocomplete(
            "cl<cursor>",
            &mos6502_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_6502_it_provides_instr_comps_existing_args_1() {
        test_instruction_autocomplete(
            "ld<cursor> inputnumber, x",
            &mos6502_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_6502_it_provides_instr_comps_existing_args_2() {
        test_instruction_autocomplete(
            "s<cursor> #$30",
            &mos6502_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_hover_6502_it_provides_reg_info() {
        test_hover("sta inputnumber, <cursor>x", "X [6502]
The X and Y registers are auxiliary registers. Like the accumulator, they can be loaded directly with values, both immediatedly (as literal constants) or from memory. Additionally, they can be incremented and decremented, and their contents may be transferred to and from the acuumulator. Their main purpose is the use as index registers, where their contents is added to a base memory location, before any values are either stored to or retrieved from the resulting address, which is known as the effective address. This is commonly used for loops and table lookups at a given index, hence the name.

Width: 8 bits", &mos6502_test_config());
    }

    #[test]
    fn handle_hover_6502_it_provides_instr_info_1() {
        test_hover(
            "c<cursor>lc",
            "clc [6502]
Clear Carry Flag
0 -> C
NZCIDV
`--0---`

## Templates

 + `CLC`

More info: https://www.masswerk.at/6502/6502_instruction_set.html#CLC",
            &mos6502_test_config(),
        );
    }

    #[test]
    fn handle_hover_6502_it_provides_instr_info_2() {
        test_hover(
            "jsr<cursor> CHRIN",
            "jsr [6502]
Jump to New Location Saving Return Address
push (PC+2),operand 1st byte -> PCLoperand 2nd byte -> PCH
NZCIDV
`------`

## Templates

 + `JSR oper`

More info: https://www.masswerk.at/6502/6502_instruction_set.html#JSR",
            &mos6502_test_config(),
        );
    }

    #[test]
    fn handle_hover_6502_it_provides_instr_info_3() {
        test_hover(
            "<cursor>sbc #$30",
            "sbc [6502]
Subtract Memory from Accumulator with Borrow
A - M - C̅ -> A
NZCIDV
`+++--+`

## Templates

 + `SBC #oper`
 + `SBC oper`
 + `SBC oper,X`
 + `SBC oper`
 + `SBC oper,X`
 + `SBC oper,Y`
 + `SBC (oper,X)`
 + `SBC (oper),Y`

More info: https://www.masswerk.at/6502/6502_instruction_set.html#SBC",
            &mos6502_test_config(),
        );
    }

    /**************************************************************************
     * RISCV Tests
     *************************************************************************/
    #[test]
    fn handle_autocomplete_riscv_it_provides_reg_comps_in_existing_reg_arg() {
        test_register_autocomplete(
            "addi a0, x<cursor>, 1",
            &riscv_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_riscv_it_provides_instr_comps_one_character_start() {
        test_instruction_autocomplete(
            "a<cursor>",
            &riscv_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_hover_riscv_it_provides_instr_info_args() {
        test_hover("<cursor>addi a0, x0, 1", "addi [riscv]
add immediate

Adds the sign-extended 12-bit immediate to register rs1. Arithmetic overflow is ignored and the result is simply the low XLEN bits of the result. ADDI rd, rs1, 0 is used to implement the MV rd, rs1 assembler pseudo-instruction.

## Templates

 + `addi       rd,rs1,imm`",
 &riscv_test_config(),
 );
    }

    #[test]
    fn handle_hover_riscv_it_provides_reg_info() {
        test_hover(
            "addi a0, x<cursor>0, 1",
            "X0 [riscv]
Hard-wired zero

Type: General Purpose Register",
            &riscv_test_config(),
        );
    }

    /**************************************************************************
     * ARM Tests
     *************************************************************************/
    #[test]
    fn handle_autocomplete_arm_it_provides_reg_comps_in_existing_reg_arg() {
        test_register_autocomplete(
            "    mov  r<cursor>, #4",
            &arm_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_arm_it_provides_instr_comps_one_character_start() {
        test_instruction_autocomplete(
            "m<cursor>",
            &arm_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    /**************************************************************************
     * Misc Tests
     *************************************************************************/
    // Labels
    #[test]
    fn handle_autocomplete_it_provides_label_comps_as_instruction_arg() {
        test_label_autocomplete(
            r"
foo:
        mov eax, 0

bar:
        call f<cursor>
            ",
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_hover_gas_it_provides_label_data_1() {
        test_hover(
            r#".LC<cursor>O:
    .string "(a & 0x0F): "
            "#,
            r#"`.string "(a & 0x0F): "`"#,
            &gas_test_config(),
        );
    }
    #[test]
    fn handle_hover_gas_it_provides_label_data_2() {
        test_hover(
            r"data_ite<cursor>ms:
	.long 1, 2, 3
            ",
            r"`.long 1, 2, 3`",
            &gas_test_config(),
        );
    }
    #[test]
    fn handle_hover_gas_it_provides_label_data_3() {
        test_hover(
            r"data_ite<cursor>ms:
	.float 1.1, 2.2, 3.3
            ",
            r"`.float 1.1, 2.2, 3.3`",
            &gas_test_config(),
        );
    }

    // Demangling
    #[test]
    fn handle_hover_it_demangles_cpp_1() {
        test_hover(
            "	call	<cursor>_ZStlsISt11char_traitsIcEERSt13basic_ostreamIcT_ES5_PKc@PLT",
            "std::basic_ostream<char, std::char_traits<char> >& std::operator<< <std::char_traits<char> >(std::basic_ostream<char, std::char_traits<char> >&, char const*)",
            &empty_test_config(),
        );
    }

    #[test]
    fn handle_hover_it_demangles_cpp_2() {
        test_hover(
            "	leaq	_ZSt4c<cursor>out(%rip), %rdi",
            "std::cout",
            &empty_test_config(),
        );
    }

    #[test]
    fn handle_hover_it_demangles_cpp_3() {
        test_hover(
            "	movq	_ZSt4endlIcSt<cursor>11char_traitsIcEERSt13basic_ostreamIT_T0_ES6_@GOTPCREL(%rip), %rax",
            "std::basic_ostream<char, std::char_traits<char> >& std::endl<char, std::char_traits<char> >(std::basic_ostream<char, std::char_traits<char> >&)",
            &empty_test_config(),
        );
    }

    /**************************************************************************
     * x86/x86-64 Tests
     *************************************************************************/
    #[test]
    fn handle_autocomplete_x86_x86_64_it_provides_instr_comps_one_character_start() {
        test_instruction_autocomplete(
            "s<cursor>",
            &x86_x86_64_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_x86_x86_64_it_provides_reg_comps_after_percent_symbol() {
        test_register_autocomplete(
            "pushq %<cursor>",
            &x86_x86_64_test_config(),
            CompletionTriggerKind::TRIGGER_CHARACTER,
            Some("%".to_string()),
        );
    }
    #[test]
    fn handle_autocomplete_x86_x86_64_it_provides_reg_comps_in_existing_reg_arg_1() {
        test_register_autocomplete(
            "pushq %rb<cursor>",
            &x86_x86_64_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_x86_x86_64_it_provides_reg_comps_in_existing_reg_arg_2() {
        test_register_autocomplete(
            "	movq	%rs<cursor>, %rbp",
            &x86_x86_64_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_x86_x86_64_it_provides_reg_comps_in_existing_reg_arg_3() {
        test_register_autocomplete(
            "	movq	%rsp, %rb<cursor>",
            &x86_x86_64_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_x86_x86_64_it_provides_reg_comps_in_existing_offset_arg() {
        test_register_autocomplete(
            "	movl	%edi, -20(%r<cursor>)",
            &x86_x86_64_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_x86_x86_64_it_provides_reg_comps_in_existing_relative_addressing_arg() {
        test_register_autocomplete(
            "	leaq	_ZSt4cout(%ri<cursor>), %rdi",
            &x86_x86_64_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_hover_x86_x86_64_it_provides_instr_info_no_args() {
        test_hover(
            "<cursor>MOVLPS",
            "movlps [x86]
Move Low Packed Single-Precision Floating-Point Values

## Forms

- *GAS*: movlps | *XMM*: SSE | *ISA*: SSE

  + [xmm]    input = true   output = true
  + [m64]    input = true   output = false
- *GAS*: movlps | *XMM*: SSE | *ISA*: SSE

  + [m64]    input = false  output = true
  + [xmm]    input = true   output = false

movlps [x86-64]
Move Low Packed Single-Precision Floating-Point Values

## Forms

- *GAS*: movlps | *XMM*: SSE | *ISA*: SSE

  + [xmm]    input = true   output = true
  + [m64]    input = true   output = false
- *GAS*: movlps | *XMM*: SSE | *ISA*: SSE

  + [m64]    input = false  output = true
  + [xmm]    input = true   output = false",
            &x86_x86_64_test_config(),
        ); // More info: https://www.felixcloutier.com/x86/movlps
    }

    #[test]
    fn handle_hover_x86_x86_64_it_provides_instr_info_one_reg_arg() {
        test_hover(
            "push<cursor>q	%rbp",
            "push [x86]
Push Value Onto the Stack

## Forms

- *GAS*: pushq

  + [imm8]   extended-size = 4
- *GAS*: pushq

  + [imm32]
- *GAS*: pushw

  + [r16]    input = true   output = false
- *GAS*: pushl

  + [r32]    input = true   output = false
- *GAS*: pushw

  + [m16]    input = true   output = false
- *GAS*: pushl

  + [m32]    input = true   output = false

push [x86-64]
Push Value Onto the Stack

## Forms

- *GAS*: pushq

  + [imm8]   extended-size = 8
- *GAS*: pushq

  + [imm32]  extended-size = 8
- *GAS*: pushw

  + [r16]    input = true   output = false
- *GAS*: pushq

  + [r64]    input = true   output = false
- *GAS*: pushw

  + [m16]    input = true   output = false
- *GAS*: pushq

  + [m64]    input = true   output = false",
            &x86_x86_64_test_config(),
        ); // More info: https://www.felixcloutier.com/x86/push
    }

    #[test]
    fn handle_hover_x86_x86_64_it_provides_instr_info_two_reg_args() {
        test_hover(
            "	m<cursor>ovq	%rsp, %rbp",
            "movq [x86]
Move Quadword

## Forms

- *GAS*: movq | *MMX*: MMX | *ISA*: MMX

  + [mm]     input = false  output = true
  + [mm]     input = true   output = false
- *GAS*: movq | *MMX*: MMX | *ISA*: MMX

  + [mm]     input = false  output = true
  + [m64]    input = true   output = false
- *GAS*: movq | *XMM*: SSE | *ISA*: SSE2

  + [xmm]    input = false  output = true
  + [xmm]    input = true   output = false
- *GAS*: movq | *XMM*: SSE | *ISA*: SSE2

  + [xmm]    input = false  output = true
  + [m64]    input = true   output = false
- *GAS*: movq | *MMX*: MMX | *ISA*: MMX

  + [m64]    input = false  output = true
  + [mm]     input = true   output = false
- *GAS*: movq | *XMM*: SSE | *ISA*: SSE2

  + [m64]    input = false  output = true
  + [xmm]    input = true   output = false

movq [x86-64]
Move Quadword

## Forms

- *GAS*: movq | *MMX*: MMX | *ISA*: MMX

  + [r64]    input = false  output = true
  + [mm]     input = true   output = false
- *GAS*: movq | *XMM*: SSE | *ISA*: SSE2

  + [r64]    input = false  output = true
  + [xmm]    input = true   output = false
- *GAS*: movq | *MMX*: MMX | *ISA*: MMX

  + [mm]     input = false  output = true
  + [r64]    input = true   output = false
- *GAS*: movq | *MMX*: MMX | *ISA*: MMX

  + [mm]     input = false  output = true
  + [mm]     input = true   output = false
- *GAS*: movq | *MMX*: MMX | *ISA*: MMX

  + [mm]     input = false  output = true
  + [m64]    input = true   output = false
- *GAS*: movq | *XMM*: SSE | *ISA*: SSE2

  + [xmm]    input = false  output = true
  + [r64]    input = true   output = false
- *GAS*: movq | *XMM*: SSE | *ISA*: SSE2

  + [xmm]    input = false  output = true
  + [xmm]    input = true   output = false
- *GAS*: movq | *XMM*: SSE | *ISA*: SSE2

  + [xmm]    input = false  output = true
  + [m64]    input = true   output = false
- *GAS*: movq | *MMX*: MMX | *ISA*: MMX

  + [m64]    input = false  output = true
  + [mm]     input = true   output = false
- *GAS*: movq | *XMM*: SSE | *ISA*: SSE2

  + [m64]    input = false  output = true
  + [xmm]    input = true   output = false",
            &x86_x86_64_test_config(),
        ); // More info: https://www.felixcloutier.com/x86/movq
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
            &x86_x86_64_test_config(),
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
            &x86_x86_64_test_config(),
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
            &x86_x86_64_test_config(),
        );
    }

    /**************************************************************************
     * GAS Tests
     *************************************************************************/
    #[test]
    fn handle_autocomplete_gas_it_provides_directive_completes_1() {
        test_directive_autocomplete(
            "	.fi<cursor>",
            &gas_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_gas_it_provides_directive_completes_2() {
        test_directive_autocomplete(
            r#"	.fil<cursor>	"a.cpp""#,
            &gas_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_gas_it_provides_directive_completes_3() {
        test_directive_autocomplete(
            ".<cursor>",
            &gas_test_config(),
            CompletionTriggerKind::TRIGGER_CHARACTER,
            Some(".".to_string()),
        );
    }

    #[test]
    fn handle_hover_gas_it_provides_directive_info_1() {
        test_hover(r#"	.f<cursor>ile	"a.cpp"#, ".file [gas]
This version of the `.file` directive tells `as` that we are about to start a new logical file. When emitting DWARF2 line number information, `.file` assigns filenames to the `.debug_line` file name table.

- .file *string*
- .file *fileno filename*

More info: https://sourceware.org/binutils/docs-2.41/as/File.html",
            &gas_test_config(),
            );
    }
    #[test]
    fn handle_hover_gas_it_provides_directive_info_2() {
        test_hover(".<cursor>text", ".text [gas]
Tells *as* to assemble the following statements onto the end of the text subsection numbered *subsection*, which is an absolute expression. If *subsection* is omitted, subsection number zero is used.

- .text *subsection*

More info: https://sourceware.org/binutils/docs-2.41/as/Text.html",
            &gas_test_config(),
            );
    }
    #[test]
    fn handle_hover_gas_it_provides_directive_info_3() {
        test_hover("	.glob<cursor>l	main", ".globl [gas]
`.globl` makes the symbol visible to `ld`. If you define symbol in your partial program, its value is made available to other partial programs that are linked with it.

- .globl *symbol*

More info: https://sourceware.org/binutils/docs-2.41/as/Global.html",
            &gas_test_config(),
            );
    }

    /**************************************************************************
     * MASM Tests
     *************************************************************************/
    #[test]
    fn handle_autocomplete_masm_it_provides_directive_completes_1() {
        test_directive_autocomplete(
            r"	ADD<cursor>",
            &masm_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }
    #[test]
    fn handle_autocomplete_masm_it_provides_directive_completes_2() {
        test_directive_autocomplete(
            ".ALLOC<cursor>",
            &masm_test_config(),
            CompletionTriggerKind::TRIGGER_CHARACTER,
            Some(".".to_string()),
        );
    }
    #[test]
    fn handle_autocomplete_masm_it_provides_directive_completes_3() {
        test_directive_autocomplete(
            ".<cursor>",
            &masm_test_config(),
            CompletionTriggerKind::TRIGGER_CHARACTER,
            Some(".".to_string()),
        );
    }

    #[test]
    fn handle_hover_masm_it_provides_directive_info_1() {
        test_hover(
            "add<cursor>R",
            "addr [masm]
Operator used exclusively with INVOKE to pass the address of a variable to a procedure.",
            &masm_test_config(),
        );
    }

    #[test]
    fn handle_hover_masm_it_provides_directive_info_2() {
        test_hover(
            "add<cursor>r",
            "addr [masm]
Operator used exclusively with INVOKE to pass the address of a variable to a procedure.",
            &masm_test_config(),
        );
    }
    #[test]
    fn handle_hover_masm_it_provides_directive_info_3() {
        test_hover(
            ".alloc<cursor>STACK",
            ".allocstack [masm]
MASM64: Generates a UWOP_ALLOC_SMALL or a UWOP_ALLOC_LARGE with the specified size for the current offset in the prologue.",
            &masm_test_config(),
        );
    }

    /**************************************************************************
     * NASM Tests
     *************************************************************************/
    #[test]
    fn handle_autocomplete_nasm_it_provides_directive_completes_1() {
        test_directive_autocomplete(
            r"	EQ<cursor>",
            &nasm_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_nasm_it_provides_directive_completes_2() {
        test_directive_autocomplete(
            "%DEF<cursor>",
            &nasm_test_config(),
            CompletionTriggerKind::TRIGGER_CHARACTER,
            Some("%".to_string()),
        );
    }

    #[test]
    fn handle_autocomplete_nasm_it_provides_directive_completes_3() {
        test_directive_autocomplete(
            "%<cursor>",
            &nasm_test_config(),
            CompletionTriggerKind::TRIGGER_CHARACTER,
            Some("%".to_string()),
        );
    }

    #[test]
    fn handle_hover_nasm_it_provides_directive_info_1() {
        test_hover(
            "EQ<cursor>U",
            "equ [nasm]
EQU defines a symbol to a given constant value: when EQU is used, the source line must contain a label. The action of EQU is to define the given label name to the value of its (only) operand. This definition is absolute, and cannot change later.",
            &nasm_test_config(),
        );
    }

    #[test]
    fn handle_hover_nasm_it_provides_directive_info_2() {
        test_hover(
            "%def<cursor>ine",
            "%define [nasm]
Define Single-line macros that is resolved at the time the embedded macro is expanded.",
            &nasm_test_config(),
        );
    }
    #[test]
    fn handle_hover_nasm_it_provides_directive_info_3() {
        test_hover(
            ".ATT_<cursor>SYNTAX",
            ".att_syntax [nasm]
switch to AT&amp;T syntax",
            &nasm_test_config(),
        );
    }

    /**************************************************************************
     * z80 Tests
     *************************************************************************/
    #[test]
    fn handle_autocomplete_z80_it_provides_instr_comps_one_character_start() {
        test_instruction_autocomplete(
            "L<cursor>",
            &z80_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_z80_it_provides_reg_comps_in_existing_reg_arg_1() {
        test_register_autocomplete(
            "LD A<cursor>",
            &z80_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_z80_it_provides_reg_comps_in_existing_reg_arg_2() {
        test_register_autocomplete(
            "        LD H<cursor>, DATA     ;STARTING ADDRESS OF DATA STRING",
            &z80_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
        );
    }

    #[test]
    fn handle_autocomplete_z80_it_provides_reg_comps_in_existing_reg_arg_3() {
        test_register_autocomplete(
            "        CP (H<cursor>)         ;COMPARE MEMORY CONTENTS WITH",
            &z80_test_config(),
            CompletionTriggerKind::INVOKED,
            None,
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
&z80_test_config(),
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
&z80_test_config(),
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
",

&z80_test_config(),
            );
    }

    #[test]
    fn handle_hover_z80_it_provides_reg_info_normal() {
        test_hover(
            "        LD H<cursor>L, DATA     ;STARTING ADDRESS OF DATA STRING",
            "HL [z80]
16-bit accumulator/address register or two 8-bit registers.

Width: 16 bits",
            &z80_test_config(),
        );
    }
    #[test]
    fn handle_hover_z80_it_provides_reg_info_prime() {
        test_hover(
            "        LD A<cursor>', '$'      ;STRING DELIMITER CODE",
            "A [z80]
Accumulator.

Width: 8 bits",
            &z80_test_config(),
        );
    }

    /**************************************************************************
     * Serialization Tests
     *************************************************************************/
    macro_rules! serialized_registers_test {
        ($serialized_path:literal, $raw_path:literal, $populate_fn:expr) => {
            let mut cmp_map = HashMap::new();
            let regs_ser = include_bytes!($serialized_path);
            let ser_vec =
                bincode::borrow_decode_from_slice::<Vec<Register>, _>(regs_ser, BINCODE_CFG)
                    .unwrap()
                    .0;

            let regs_raw = include_str!($raw_path);
            let mut raw_vec = $populate_fn(regs_raw).unwrap();

            // HACK: Windows line endings...
            if cfg!(target_os = "windows") {
                for reg in &mut raw_vec {
                    if let Some(descr) = &reg.description {
                        reg.description = Some(descr.replace('\r', ""));
                    }
                }
            }

            for reg in ser_vec {
                *cmp_map.entry(reg.clone()).or_insert(0) += 1;
            }
            for reg in raw_vec {
                let entry = cmp_map.get_mut(&reg).unwrap();
                assert!(
                    *entry != 0,
                    "Expected at least one more instruction entry for {reg:?}, but the count is 0"
                );
                *entry -= 1;
            }
            for (reg, count) in &cmp_map {
                assert!(
                    *count == 0,
                    "Expected count to be 0, found {count} for {reg:?}"
                );
            }
        };
    }
    #[test]
    fn serialized_x86_registers_are_up_to_date() {
        serialized_registers_test!(
            "serialized/registers/x86",
            "../docs_store/registers/x86.xml",
            populate_registers
        );
    }
    #[test]
    fn serialized_x86_64_registers_are_up_to_date() {
        serialized_registers_test!(
            "serialized/registers/x86_64",
            "../docs_store/registers/x86_64.xml",
            populate_registers
        );
    }
    #[test]
    fn serialized_arm_registers_are_up_to_date() {
        serialized_registers_test!(
            "serialized/registers/arm",
            "../docs_store/registers/arm.xml",
            populate_registers
        );
    }
    #[test]
    fn serialized_arm64_registers_are_up_to_date() {
        serialized_registers_test!(
            "serialized/registers/arm64",
            "../docs_store/registers/arm64.xml",
            populate_registers
        );
    }
    #[test]
    fn serialized_z80_registers_are_up_to_date() {
        serialized_registers_test!(
            "serialized/registers/z80",
            "../docs_store/registers/z80.xml",
            populate_registers
        );
    }
    #[test]
    fn serialized_riscv_registers_are_up_to_date() {
        serialized_registers_test!(
            "serialized/registers/riscv",
            "../docs_store/registers/riscv.rst.txt",
            populate_riscv_registers
        );
    }
    #[test]
    fn serialized_6502_registers_are_up_to_date() {
        serialized_registers_test!(
            "serialized/registers/6502",
            "../docs_store/registers/6502.xml",
            populate_registers
        );
    }
    #[test]
    fn serialized_power_isa_registers_are_up_to_date() {
        serialized_registers_test!(
            "serialized/registers/power-isa",
            "../docs_store/registers/power-isa.xml",
            populate_registers
        );
    }
    #[test]
    fn serialized_avr_registers_are_up_to_date() {
        serialized_registers_test!(
            "serialized/registers/avr",
            "../docs_store/registers/avr.xml",
            populate_registers
        );
    }
    #[test]
    fn serialized_mips_registers_are_up_to_date() {
        serialized_registers_test!(
            "serialized/registers/mips",
            "../docs_store/registers/mips.xml",
            populate_registers
        );
    }

    macro_rules! serialized_instructions_test {
        ($serialized_path:literal, $raw_path:literal, $populate_fn:expr) => {
            let mut cmp_map = HashMap::new();
            let instrs_ser = include_bytes!($serialized_path);
            let mut ser_vec =
                bincode::borrow_decode_from_slice::<Vec<Instruction>, _>(instrs_ser, BINCODE_CFG)
                    .unwrap()
                    .0;
            let instrs_raw = include_str!($raw_path);
            let mut raw_vec = $populate_fn(instrs_raw).unwrap();

            // HACK: Windows line endings...
            if cfg!(target_os = "windows") {
                for instr in &mut raw_vec {
                    instr.summary = instr.summary.replace('\r', "");
                    for form in &mut instr.forms {
                        if let Some(descr) = &form.avr_summary {
                            form.avr_summary = Some(descr.replace('\r', ""));
                        }
                    }
                }
            }

            // HACK: To work around the difference in extra info urls between testing
            // and production
            for instr in &mut ser_vec {
                instr.url = None;
            }
            for instr in &mut raw_vec {
                instr.url = None;
            }

            for instr in ser_vec {
                *cmp_map.entry(instr.clone()).or_insert(0) += 1;
            }
            for instr in raw_vec {
                let entry = cmp_map.get_mut(&instr).unwrap();
                assert!(
                    *entry != 0,
                    "Expected at least one more instruction entry for {instr:?}, but the count is 0"
                );
                *entry -= 1;
            }
            for (instr, count) in &cmp_map {
                assert!(
                    *count == 0,
                    "Expected count to be 0, found {count} for {instr:?}"
                );
            }
        };
    }

    #[test]
    fn serialized_x86_instructions_are_up_to_date() {
        serialized_instructions_test!(
            "serialized/opcodes/x86",
            "../docs_store/opcodes/x86.xml",
            populate_instructions
        );
    }
    #[test]
    fn serialized_x86_64_instructions_are_up_to_date() {
        serialized_instructions_test!(
            "serialized/opcodes/x86_64",
            "../docs_store/opcodes/x86_64.xml",
            populate_instructions
        );
    }
    #[test]
    fn serialized_z80_instructions_are_up_to_date() {
        serialized_instructions_test!(
            "serialized/opcodes/z80",
            "../docs_store/opcodes/z80.xml",
            populate_instructions
        );
    }
    #[test]
    fn serialized_6502_instructions_are_up_to_date() {
        serialized_instructions_test!(
            "serialized/opcodes/6502",
            "../docs_store/opcodes/6502.html",
            populate_6502_instructions
        );
    }
    #[test]
    fn serialized_power_isa_instructions_are_up_to_date() {
        serialized_instructions_test!(
            "serialized/opcodes/power-isa",
            "../docs_store/opcodes/power-isa.json",
            populate_power_isa_instructions
        );
    }
    #[test]
    fn serialized_avr_instructions_are_up_to_date() {
        serialized_instructions_test!(
            "serialized/opcodes/avr",
            "../docs_store/opcodes/avr.xml",
            populate_avr_instructions
        );
    }
    #[test]
    fn serialized_mips_instructions_are_up_to_date() {
        serialized_instructions_test!(
            "serialized/opcodes/mips",
            "../docs_store/opcodes/mips.json",
            populate_mips_instructions
        );
    }
    #[test]
    fn serialized_mars_pseudo_instructions_are_up_to_date() {
        serialized_instructions_test!(
            "serialized/opcodes/mars",
            "../docs_store/opcodes/mars.txt",
            populate_mars_pseudo_instructions
        );
    }
    // TODO: Consolidate this into `serialized_instruction_test!`
    // TODO: sperate test for aarch64 when the arm32 opcodes are added
    #[test]
    fn serialized_arm_instructions_are_up_to_date() {
        let mut cmp_map = HashMap::new();
        let arm_instrs_ser = include_bytes!("serialized/opcodes/arm");
        let mut ser_vec =
            bincode::borrow_decode_from_slice::<Vec<Instruction>, _>(arm_instrs_ser, BINCODE_CFG)
                .unwrap()
                .0;
        ser_vec.sort_by(|a, b| a.name.cmp(&b.name));

        let mut raw_vec =
            populate_arm_instructions(&PathBuf::from("../docs_store/opcodes/ARM/")).unwrap();
        raw_vec.sort_by(|a, b| a.name.cmp(&b.name));

        for instr in ser_vec {
            *cmp_map.entry(instr.clone()).or_insert(0) += 1;
        }
        for instr in raw_vec {
            let entry = cmp_map.get_mut(&instr).unwrap();
            assert!(
                *entry != 0,
                "Expected at least one more instruction entry for {instr:?}, but the count is 0"
            );
            *entry -= 1;
        }
        for (instr, count) in &cmp_map {
            assert!(
                *count == 0,
                "Expected count to be 0, found {count} for {instr:?}"
            );
        }
    }

    // TODO: Consolidate this into `serialized_instruction_test!`
    #[test]
    fn serialized_riscv_instructions_are_up_to_date() {
        let mut cmp_map = HashMap::new();
        let riscv_instrs_ser = include_bytes!("serialized/opcodes/riscv");
        let ser_vec =
            bincode::borrow_decode_from_slice::<Vec<Instruction>, _>(riscv_instrs_ser, BINCODE_CFG)
                .unwrap()
                .0;

        let raw_vec =
            populate_riscv_instructions(&PathBuf::from("../docs_store/opcodes/RISCV/")).unwrap();

        for instr in ser_vec {
            *cmp_map.entry(instr.clone()).or_insert(0) += 1;
        }
        for instr in raw_vec {
            let entry = cmp_map.get_mut(&instr).unwrap();
            assert!(
                *entry != 0,
                "Expected at least one more instruction entry for {instr:?}, but the count is 0"
            );
            *entry -= 1;
        }
        for (instr, count) in &cmp_map {
            assert!(
                *count == 0,
                "Expected count to be 0, found {count} for {instr:?}"
            );
        }
    }

    macro_rules! serialized_directives_test {
        ($serialized_path:literal, $raw_path:literal, $populate_fn:expr) => {
            let mut cmp_map = HashMap::new();
            let dirs_ser = include_bytes!($serialized_path);
            let ser_vec =
                bincode::borrow_decode_from_slice::<Vec<Directive>, _>(dirs_ser, BINCODE_CFG)
                    .unwrap()
                    .0;

            let dirs_raw = include_str!($raw_path);
            let mut raw_vec = $populate_fn(dirs_raw).unwrap();

            // HACK: Windows line endings...
            if cfg!(target_os = "windows") {
                for dir in &mut raw_vec {
                    dir.description = dir.description.replace('\r', "");
                }
            }

            for dir in ser_vec {
                *cmp_map.entry(dir.clone()).or_insert(0) += 1;
            }
            for dir in raw_vec {
                let entry = cmp_map.get_mut(&dir).unwrap();
                assert!(
                    *entry != 0,
                    "Expected at least one more instruction entry for {dir:?}, but the count is 0"
                );
                *entry -= 1;
            }
            for (dir, count) in &cmp_map {
                assert!(
                    *count == 0,
                    "Expected count to be 0, found {count} for {dir:?}"
                );
            }
        };
    }
    #[test]
    fn serialized_gas_directives_are_up_to_date() {
        serialized_directives_test!(
            "serialized/directives/gas",
            "../docs_store/directives/gas.xml",
            populate_gas_directives
        );
    }
    #[test]
    fn serialized_masm_directives_are_up_to_date() {
        serialized_directives_test!(
            "serialized/directives/masm",
            "../docs_store/directives/masm.xml",
            populate_masm_nasm_fasm_mars_directives
        );
    }
    #[test]
    fn serialized_nasm_directives_are_up_to_date() {
        serialized_directives_test!(
            "serialized/directives/nasm",
            "../docs_store/directives/nasm.xml",
            populate_masm_nasm_fasm_mars_directives
        );
    }
    #[test]
    fn serialized_ca65_directives_are_up_to_date() {
        serialized_directives_test!(
            "serialized/directives/ca65",
            "../docs_store/directives/ca65.html",
            populate_ca65_directives
        );
    }
    #[test]
    fn serialized_avr_directives_are_up_to_date() {
        serialized_directives_test!(
            "serialized/directives/avr",
            "../docs_store/directives/avr.xml",
            populate_avr_directives
        );
    }
    #[test]
    fn serialized_fasm_directives_are_up_to_date() {
        serialized_directives_test!(
            "serialized/directives/fasm",
            "../docs_store/directives/fasm.xml",
            populate_masm_nasm_fasm_mars_directives
        );
    }
    #[test]
    fn serialized_mars_directives_are_up_to_date() {
        serialized_directives_test!(
            "serialized/directives/mars",
            "../docs_store/directives/mars.xml",
            populate_masm_nasm_fasm_mars_directives
        );
    }
}
