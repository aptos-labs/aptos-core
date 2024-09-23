// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{CliCommand, CliError, CliTypedResult, PromptOptions},
        utils::{
            check_if_file_exists, create_dir_if_not_exist, read_dir_files, read_from_file,
            write_to_user_only_file,
        },
    },
    update::get_revela_path,
};
use anyhow::Context;
use async_trait::async_trait;
use clap::{Args, Parser};
use itertools::Itertools;
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
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const DISASSEMBLER_EXTENSION: &str = "mv.asm";
const DECOMPILER_EXTENSION: &str = "mv.move";

/// Disassemble the Move bytecode pointed to in the textual representation
/// of Move bytecode.
///
/// For example, if you want to disassemble an on-chain package `PackName` at account `0x42`:
/// 1. Download the package with `aptos move download --account 0x42 --package PackName --bytecode`
/// 2. Disassemble the package bytecode with `aptos move disassemble --package-path PackName/bytecode_modules`
#[derive(Debug, Parser)]
pub struct Disassemble {
    #[clap(flatten)]
    pub command: BytecodeCommand,
}

/// Decompile the Move bytecode pointed to into Move source code.
///
/// For example, if you want to decompile an on-chain package `PackName` at account `0x42`:
/// 1. Download the package with `aptos move download --account 0x42 --package PackName --bytecode`
/// 2. Decompile the package bytecode with `aptos decompile --package-path PackName/bytecode_modules`
#[derive(Debug, Parser)]
pub struct Decompile {
    #[clap(flatten)]
    pub command: BytecodeCommand,
}

#[derive(Debug, Args)]
pub struct BytecodeCommand {
    /// Treat input file as a script (default is to treat file as a module)
    #[clap(long)]
    pub is_script: bool,

    #[clap(flatten)]
    input: BytecodeCommandInput,

    /// (Optional) Currently only for disassemble: path to a coverage file for the VM in order
    /// to print trace information in the disassembled output.
    #[clap(long)]
    pub code_coverage_path: Option<PathBuf>,

    /// Output directory for the generated file. Defaults to the directory of the
    /// `path/module.mv` file if not provided. The disassembled output is stored
    /// at `output_dir/module.mv.asm`, and decompiled output at
    /// `output_dir/module.mv.move`.
    #[clap(long, value_parser)]
    pub(crate) output_dir: Option<PathBuf>,

    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
}

/// Allows to ensure that either one of both is selected (via  the `group` attribute).
#[derive(Debug, Args)]
#[group(required = true, multiple = false)]
pub struct BytecodeCommandInput {
    /// The path to a directory containing Move bytecode files with the extension `.mv`.
    /// The tool will process all files find in this directory
    ///
    /// If present, a source map at the same location ending in `.mvsm` and the source
    /// file itself ending in`.move` will be processed by the tool.
    #[clap(long)]
    pub package_path: Option<PathBuf>,

    /// Alternatively to a package path, path to a single bytecode file which should be processed.
    #[clap(long)]
    pub bytecode_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
enum BytecodeCommandType {
    Disassemble,
    Decompile,
}

#[async_trait]
impl CliCommand<String> for Disassemble {
    fn command_name(&self) -> &'static str {
        "Disassemble"
    }

    async fn execute(mut self) -> CliTypedResult<String> {
        self.command.execute(BytecodeCommandType::Disassemble).await
    }
}

#[async_trait]
impl CliCommand<String> for Decompile {
    fn command_name(&self) -> &'static str {
        "Decompile"
    }

    async fn execute(mut self) -> CliTypedResult<String> {
        self.command.execute(BytecodeCommandType::Decompile).await
    }
}

impl BytecodeCommand {
    async fn execute(self, command_type: BytecodeCommandType) -> CliTypedResult<String> {
        let inputs = if let Some(path) = self.input.bytecode_path.clone() {
            vec![path]
        } else if let Some(path) = self.input.package_path.clone() {
            read_dir_files(path.as_path(), |p| {
                p.extension()
                    .map(|s| s == MOVE_COMPILED_EXTENSION)
                    .unwrap_or_default()
            })?
        } else {
            unreachable!("arguments required by clap")
        };

        let mut report = vec![];
        let mut last_out_dir = String::new();
        for bytecode_path in inputs {
            let bytecode_path = bytecode_path.as_path();
            let extension = bytecode_path
                .extension()
                .context("Missing file extension for bytecode file")?;
            if extension != MOVE_COMPILED_EXTENSION {
                return Err(CliError::UnexpectedError(format!(
                    "Bad source file extension {:?}; expected {}",
                    extension, MOVE_COMPILED_EXTENSION
                )));
            }

            let (output, extension) = match command_type {
                BytecodeCommandType::Disassemble => {
                    (self.disassemble(bytecode_path)?, DISASSEMBLER_EXTENSION)
                },
                BytecodeCommandType::Decompile => {
                    (self.decompile(bytecode_path)?, DECOMPILER_EXTENSION)
                },
            };

            let output_dir = if let Some(dir) = self.output_dir.clone() {
                dir
            } else {
                bytecode_path.parent().expect("has parent dir").to_owned()
            };
            last_out_dir = output_dir.display().to_string();

            let output_file = output_dir
                .join(bytecode_path.file_name().expect("file name"))
                .with_extension(extension);
            check_if_file_exists(output_file.as_path(), self.prompt_options)?;

            // Create the directory if it doesn't exist
            create_dir_if_not_exist(output_dir.as_path())?;

            // write to file
            write_to_user_only_file(
                output_file.as_path(),
                &output_file.display().to_string(),
                output.as_bytes(),
            )?;
            report.push(
                output_file
                    .file_name()
                    .expect("file name")
                    .to_string_lossy()
                    .to_string(),
            );
        }

        Ok(match report.len() {
            0 => "no bytecode modules found".to_owned(),
            1 => format!("{}/{}", last_out_dir, report[0]),
            _ => format!("{}/{{{}}}", last_out_dir, report.into_iter().join(",")),
        })
    }

    fn disassemble(&self, bytecode_path: &Path) -> Result<String, CliError> {
        let bytecode_bytes = read_from_file(bytecode_path)?;
        let move_path = bytecode_path.with_extension(MOVE_EXTENSION);
        let source_map_path = bytecode_path.with_extension(SOURCE_MAP_EXTENSION);

        let source = fs::read_to_string(move_path).ok();
        let source_map = source_map_from_file(&source_map_path).ok();

        let disassembler_options = DisassemblerOptions {
            print_code: true,
            only_externally_visible: false,
            print_basic_blocks: true,
            print_locals: true,
            print_bytecode_stats: false,
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

        let mut source_mapping = if let Some(s) = source_map {
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

        disassembler
            .disassemble()
            .map_err(|err| CliError::UnexpectedError(format!("Unable to disassemble: {}", err)))
    }

    fn decompile(&self, bytecode_path: &Path) -> Result<String, CliError> {
        let exe = get_revela_path()?;
        let to_cli_error = |e| CliError::IO(exe.display().to_string(), e);
        let mut cmd = Command::new(exe.as_path());
        cmd.arg(format!("--bytecode={}", bytecode_path.display()));
        if self.is_script {
            cmd.arg("--script");
        }
        let out = cmd.output().map_err(to_cli_error)?;
        if out.status.success() {
            String::from_utf8(out.stdout).map_err(|err| {
                CliError::UnexpectedError(format!(
                    "output generated by decompiler is not valid utf8: {}",
                    err
                ))
            })
        } else {
            Err(CliError::UnexpectedError(format!(
                "decompiler exited with status {}: {}",
                out.status,
                String::from_utf8(out.stderr).unwrap_or_default()
            )))
        }
    }
}
