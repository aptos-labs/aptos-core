// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::MoveEnv;
use anyhow::Context;
use aptos_cli_common::{
    check_if_file_exists, create_dir_if_not_exist, read_dir_files, read_from_file,
    write_to_user_only_file, CliCommand, CliError, CliTypedResult, PromptOptions,
};
use aptos_types::vm::module_metadata::prelude::*;
use async_trait::async_trait;
use clap::{Args, Parser};
use itertools::Itertools;
use move_binary_format::{file_format::CompiledScript, CompiledModule};
use move_command_line_common::files::MOVE_COMPILED_EXTENSION;
use move_coverage::coverage_map::CoverageMap;
use move_decompiler::{Decompiler, Options as DecompilerOptions};
use move_model::metadata::{CompilationMetadata, CompilerVersion, LanguageVersion};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    str,
    sync::Arc,
};

const DISASSEMBLER_EXTENSION: &str = "mv.masm";
const DECOMPILER_EXTENSION: &str = "mv.move";

/// Disassemble the Move bytecode pointed to in the textual representation
/// of Move bytecode.
///
/// For example, if you want to disassemble an on-chain package `PackName` at account `0x42`:
/// 1. Download the package with `aptos move download --account 0x42 --package PackName --bytecode`
/// 2. Disassemble the package bytecode with `aptos move disassemble --package-dir PackName/bytecode_modules`
#[derive(Debug, Parser)]
pub struct Disassemble {
    #[clap(flatten)]
    pub command: BytecodeCommand,

    #[clap(skip)]
    pub env: Arc<MoveEnv>,
}

/// Decompile the Move bytecode pointed to into Move source code.
///
/// For example, if you want to decompile an on-chain package `PackName` at account `0x42`:
/// 1. Download the package with `aptos move download --account 0x42 --package PackName --bytecode`
/// 2. Decompile the package bytecode with `aptos move decompile --package-dir PackName/bytecode_modules`
#[derive(Debug, Parser)]
pub struct Decompile {
    #[clap(flatten)]
    pub command: BytecodeCommand,

    #[clap(skip)]
    pub env: Arc<MoveEnv>,
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

    /// When `--bytecode-path` is set with this option,
    /// only print out the metadata and bytecode version of the target bytecode
    #[clap(long)]
    pub print_metadata_only: bool,
}

/// Allows to ensure that either one of both is selected (via  the `group` attribute).
#[derive(Debug, Args)]
#[group(required = true, multiple = false)]
pub struct BytecodeCommandInput {
    /// The path to a directory containing Move bytecode files with the extension `.mv`.
    /// The tool will process all files found in this directory.
    ///
    /// If present, a source map at the same location ending in `.mvsm` and the source
    /// file itself ending in `.move` will be processed by the tool.
    #[clap(long, alias = "package-path")]
    pub package_dir: Option<PathBuf>,

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct BytecodeMetadata {
    aptos_metadata: Option<RuntimeModuleMetadataV1>,
    bytecode_version: u32,
    compilation_metadata: CompilationMetadata,
}

impl BytecodeCommand {
    async fn execute(self, command_type: BytecodeCommandType) -> CliTypedResult<String> {
        let inputs = if let Some(path) = self.input.bytecode_path.clone() {
            vec![path]
        } else if let Some(path) = self.input.package_dir.clone() {
            read_dir_files(path.as_path(), |p| {
                p.extension()
                    .map(|s| s == MOVE_COMPILED_EXTENSION)
                    .unwrap_or_default()
            })?
        } else {
            unreachable!("arguments required by clap")
        };

        if self.print_metadata_only && self.input.bytecode_path.is_some() {
            return self.print_metadata(&inputs[0]);
        }

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

    fn print_metadata(&self, bytecode_path: &Path) -> Result<String, CliError> {
        let bytecode_bytes = read_from_file(bytecode_path)?;

        let v1_metadata = CompilationMetadata {
            unstable: false,
            compiler_version: CompilerVersion::V1.to_string(),
            language_version: LanguageVersion::V1.to_string(),
        };
        let metadata = if self.is_script {
            let script = CompiledScript::deserialize(&bytecode_bytes).context(format!(
                "Script blob at {} can't be deserialized",
                bytecode_path.display()
            ))?;
            if let Some(data) = get_compilation_metadata(&script) {
                serde_json::to_string_pretty(&data).expect("expect compilation metadata")
            } else {
                serde_json::to_string_pretty(&v1_metadata).expect("expect compilation metadata")
            };
            BytecodeMetadata {
                aptos_metadata: get_metadata_from_compiled_code(&script),
                bytecode_version: script.version,
                compilation_metadata: get_compilation_metadata(&script).unwrap_or(v1_metadata),
            }
        } else {
            let module = CompiledModule::deserialize(&bytecode_bytes).context(format!(
                "Module blob at {} can't be deserialized",
                bytecode_path.display()
            ))?;
            BytecodeMetadata {
                aptos_metadata: get_metadata_from_compiled_code(&module),
                bytecode_version: module.version,
                compilation_metadata: get_compilation_metadata(&module).unwrap_or(v1_metadata),
            }
        };
        println!(
            "Metadata: {}",
            serde_json::to_string_pretty(&metadata).expect("expect metadata")
        );
        Ok("ok".to_string())
    }

    fn disassemble(&self, bytecode_path: &Path) -> Result<String, CliError> {
        let bytecode_bytes = read_from_file(bytecode_path)?;
        let coverage_map = if let Some(file_path) = &self.code_coverage_path {
            Some(
                CoverageMap::from_binary_file(file_path)
                    .map_err(|_err| {
                        CliError::UnexpectedError("Unable to read from file_path".to_string())
                    })?
                    .to_unified_exec_map(),
            )
        } else {
            None
        };
        if self.is_script {
            let script = CompiledScript::deserialize(&bytecode_bytes).context(format!(
                "Script blob at {} can't be deserialized",
                bytecode_path.display()
            ))?;
            if let Some(ref cov) = coverage_map {
                Ok(move_asm::disassembler::disassemble_script_with_coverage(
                    &script, cov,
                )?)
            } else {
                Ok(move_asm::disassembler::disassemble_script(
                    String::new(),
                    &script,
                )?)
            }
        } else {
            let module = CompiledModule::deserialize(&bytecode_bytes).context(format!(
                "Module blob at {} can't be deserialized",
                bytecode_path.display()
            ))?;
            if let Some(ref cov) = coverage_map {
                Ok(move_asm::disassembler::disassemble_module_with_coverage(
                    &module, cov,
                )?)
            } else {
                Ok(move_asm::disassembler::disassemble_module(
                    String::new(),
                    &module,
                )?)
            }
        }
    }

    fn decompile(&self, bytecode_path: &Path) -> Result<String, CliError> {
        let bytecode_bytes = read_from_file(bytecode_path)?;
        let mut decompiler = Decompiler::new(DecompilerOptions::default());
        let source_map =
            decompiler.empty_source_map(&bytecode_path.to_string_lossy(), &bytecode_bytes);
        let res = if self.is_script {
            let script = CompiledScript::deserialize(&bytecode_bytes).context(format!(
                "Script blob at {} can't be deserialized",
                bytecode_path.display()
            ))?;
            decompiler.decompile_script(script, source_map)?
        } else {
            let module = CompiledModule::deserialize(&bytecode_bytes).context(format!(
                "Module blob at {} can't be deserialized",
                bytecode_path.display()
            ))?;
            decompiler.decompile_module(module, source_map)?
        };
        Ok(res)
    }
}
