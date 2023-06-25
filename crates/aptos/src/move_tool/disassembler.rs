// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    types::{CliCommand, CliError, CliTypedResult, PromptOptions},
    utils::{
        check_if_file_exists, create_dir_if_not_exist, dir_default_to_current, read_from_file,
        write_to_user_only_file,
    },
};
use anyhow::Context;
use async_trait::async_trait;
use clap::Parser;
use move_binary_format::{
    binary_views::BinaryIndexedView, file_format::CompiledScript, CompiledModule,
};
use move_bytecode_source_map::{mapping::SourceMapping, utils::source_map_from_file};
use move_command_line_common::files::{
    MOVE_COMPILED_EXTENSION, MOVE_EXTENSION, SOURCE_MAP_EXTENSION,
};
use move_coverage::coverage_map::CoverageMap;
use move_disassembler::disassembler::{Disassembler, DisassemblerOptions};
use move_ir_types::location::Spanned;
use std::{fs, path::PathBuf};

const DISASSEMBLED_CODE_FILE: &str = "disassembled-code.move";

/// Disassemble the Move bytecode pointed to
///
/// For example, if you want to disassemble on chain module:
/// 1. Download the package - aptos move download
/// 2. Compile the package - aptos move compile
/// 3. Cd to package and disassemble - aptos move disassemble --bytecode-path ./test.mv
#[derive(Debug, Parser)]
pub struct Disassemble {
    /// Treat input file as a script (default is to treat file as a module)
    #[clap(long)]
    pub is_script: bool,

    /// The path to the bytecode file to disassemble;
    ///
    /// let's call it file.mv. We assume that two other files reside under the same directory:
    /// a source map file.mvsm (possibly) and the Move source code file.move.
    #[clap(long)]
    pub bytecode_path: PathBuf,

    /// (Optional) Path to a coverage file for the VM in order to print trace information in the
    /// disassembled output.
    #[clap(long)]
    pub code_coverage_path: Option<PathBuf>,

    /// Output directory for the key files
    #[clap(long, value_parser)]
    pub(crate) output_dir: Option<PathBuf>,

    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
}

#[async_trait]
impl CliCommand<String> for Disassemble {
    fn command_name(&self) -> &'static str {
        "Disassemble"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let bytecode_path = self.bytecode_path.as_path();
        let extension = bytecode_path
            .extension()
            .context("Missing file extension for bytecode file")?;
        if extension != MOVE_COMPILED_EXTENSION {
            return Err(CliError::UnexpectedError(format!(
                "Bad source file extension {:?}; expected {}",
                extension, MOVE_COMPILED_EXTENSION
            )));
        }

        let bytecode_bytes = read_from_file(bytecode_path)?;
        let move_path = bytecode_path.with_extension(MOVE_EXTENSION);
        let source_map_path = bytecode_path.with_extension(SOURCE_MAP_EXTENSION);

        let source = fs::read_to_string(move_path).ok();
        let source_map = source_map_from_file(&source_map_path);

        let disassembler_options = DisassemblerOptions {
            print_code: true,
            only_externally_visible: false,
            print_basic_blocks: true,
            print_locals: true,
        };

        let no_loc = Spanned::unsafe_no_loc(()).loc;
        let module: CompiledModule;
        let script: CompiledScript;
        let bytecode = if self.is_script {
            script = CompiledScript::deserialize(&bytecode_bytes)
                .context("Script blob can't be deserialized")?;
            BinaryIndexedView::Script(&script)
        } else {
            module = CompiledModule::deserialize(&bytecode_bytes)
                .context("Module blob can't be deserialized")?;
            BinaryIndexedView::Module(&module)
        };

        let mut source_mapping = if let Ok(s) = source_map {
            SourceMapping::new(s, bytecode)
        } else {
            SourceMapping::new_from_view(bytecode, no_loc)
                .context("Unable to build dummy source mapping")?
        };

        if let Some(source_code) = source {
            source_mapping.with_source_code((bytecode_path.display().to_string(), source_code));
        }

        let mut disassembler = Disassembler::new(source_mapping, disassembler_options);

        if let Some(file_path) = &self.code_coverage_path {
            disassembler.add_coverage_map(
                CoverageMap::from_binary_file(file_path)
                    .map_err(|_err| {
                        CliError::UnexpectedError("Unable to read from file_path".to_string())
                    })?
                    .to_unified_exec_map(),
            );
        }

        let disassemble_string = disassembler
            .disassemble()
            .map_err(|_err| CliError::UnexpectedError("Unable to disassemble".to_string()))?;

        let output_dir = dir_default_to_current(self.output_dir.clone())?;
        let disassemble_file = output_dir.join(DISASSEMBLED_CODE_FILE);
        check_if_file_exists(disassemble_file.as_path(), self.prompt_options)?;

        // Create the directory if it doesn't exist
        create_dir_if_not_exist(output_dir.as_path())?;

        // write to file
        write_to_user_only_file(
            disassemble_file.as_path(),
            DISASSEMBLED_CODE_FILE,
            disassemble_string.as_bytes(),
        )?;

        Ok(disassemble_file.as_path().display().to_string())
    }
}
