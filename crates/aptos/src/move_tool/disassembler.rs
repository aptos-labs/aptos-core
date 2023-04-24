// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_command_line_common::files::{
    MOVE_COMPILED_EXTENSION, MOVE_EXTENSION, SOURCE_MAP_EXTENSION,
};
use crate::common::types::{CliCommand, CliTypedResult};
use std::{fs, path::Path};
use move_binary_format::binary_views::BinaryIndexedView;
use move_binary_format::CompiledModule;
use move_binary_format::file_format::CompiledScript;
use move_coverage::coverage_map::CoverageMap;
use move_disassembler::disassembler::{Disassembler, DisassemblerOptions};
use clap::Parser;
use async_trait::async_trait;
use move_bytecode_source_map::{mapping::SourceMapping, utils::source_map_from_file};
use move_ir_types::location::Spanned;

/// Disassemble the Move bytecode pointed to
#[derive(Debug, Parser)]
pub struct AptosDisassembler {
    /// Skip printing of private functions.
    #[clap(long = "skip-private")]
    pub skip_private: bool,

    /// Do not print the disassembled bytecodes of each function.
    #[clap(long = "skip-code")]
    pub skip_code: bool,

    /// Do not print locals of each function.
    #[clap(long = "skip-locals")]
    pub skip_locals: bool,

    /// Do not print the basic blocks of each function.
    #[clap(long = "skip-basic-blocks")]
    pub skip_basic_blocks: bool,

    /// Treat input file as a script (default is to treat file as a module)
    #[clap(short = 's', long = "script")]
    pub is_script: bool,

    /// The path to the bytecode file to disassemble; let's call it file.mv. We assume that two
    /// other files reside under the same directory: a source map file.mvsm (possibly) and the Move
    /// source code file.move.
    #[clap(short = 'b', long = "bytecode")]
    pub bytecode_file_path: String,

    /// (Optional) Path to a coverage file for the VM in order to print trace information in the
    /// disassembled output.
    #[clap(short = 'c', long = "move-coverage-path")]
    pub code_coverage_path: Option<String>,
}

#[async_trait]
impl CliCommand<String> for AptosDisassembler {
    fn command_name(&self) -> &'static str {
        "AptosDisassembler"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let move_extension = MOVE_EXTENSION;
        let mv_bytecode_extension = MOVE_COMPILED_EXTENSION;
        let source_map_extension = SOURCE_MAP_EXTENSION;

        let source_path = Path::new(&self.bytecode_file_path);
        let extension = source_path
            .extension()
            .expect("Missing file extension for bytecode file");
        if extension != mv_bytecode_extension {
            return Ok(format!(
                "Bad source file extension {:?}; expected {}",
                extension, mv_bytecode_extension)
            );
        }

        let bytecode_bytes = fs::read(&self.bytecode_file_path).expect("Unable to read bytecode file");

        let source_path = Path::new(&self.bytecode_file_path).with_extension(move_extension);
        let source = fs::read_to_string(&source_path).ok();
        let source_map = source_map_from_file(
            &Path::new(&self.bytecode_file_path).with_extension(source_map_extension),
        );

        let mut disassembler_options = DisassemblerOptions::new();
        disassembler_options.print_code = !self.skip_code;
        disassembler_options.only_externally_visible = self.skip_private;
        disassembler_options.print_basic_blocks = !self.skip_basic_blocks;
        disassembler_options.print_locals = !self.skip_locals;

        // TODO: make source mapping work with the Move source language
        let no_loc = Spanned::unsafe_no_loc(()).loc;
        let module: CompiledModule;
        let script: CompiledScript;
        let bytecode = if self.is_script {
            script = CompiledScript::deserialize(&bytecode_bytes)
                .expect("Script blob can't be deserialized");
            BinaryIndexedView::Script(&script)
        } else {
            module = CompiledModule::deserialize(&bytecode_bytes)
                .expect("Module blob can't be deserialized");
            BinaryIndexedView::Module(&module)
        };

        let mut source_mapping = {
            if let Ok(s) = source_map {
                SourceMapping::new(s, bytecode)
            } else {
                SourceMapping::new_from_view(bytecode, no_loc)
                    .expect("Unable to build dummy source mapping")
            }
        };

        if let Some(source_code) = source {
            source_mapping.with_source_code((source_path.to_str().unwrap().to_string(), source_code));
        }

        let mut disassembler = Disassembler::new(source_mapping, disassembler_options);

        if let Some(file_path) = &self.code_coverage_path {
            disassembler.add_coverage_map(
                CoverageMap::from_binary_file(file_path)
                    .unwrap()
                    .to_unified_exec_map(),
            );
        }

        let disassemble_string = disassembler.disassemble().expect("Unable to dissassemble");

        return Ok(disassemble_string);
    }
}
