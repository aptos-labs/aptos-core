// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Script compilation support for the `aptos move run-script` and `aptos governance` commands.

use crate::FrameworkPackageArgs;
use aptos_cli_common::{CliError, CliTypedResult, PromptOptions};
use aptos_crypto::HashValue;
use aptos_framework::{BuildOptions, BuiltPackage};
use clap::Parser;
use move_core_types::diag_writer::DiagWriter;
use move_model::metadata::{
    CompilerVersion, LanguageVersion, LATEST_STABLE_COMPILER_VERSION,
    LATEST_STABLE_LANGUAGE_VERSION,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

/// Compile a specified script.
#[derive(Debug, Parser, Default)]
pub struct CompileScriptFunction {
    /// Path to the Move script for the proposal
    #[clap(long, group = "script", value_parser)]
    pub script_path: Option<PathBuf>,

    /// Path to the Move script for the proposal
    #[clap(long, group = "script", value_parser)]
    pub compiled_script_path: Option<PathBuf>,

    #[clap(flatten)]
    pub framework_package_args: FrameworkPackageArgs,

    #[clap(long)]
    pub bytecode_version: Option<u32>,

    /// Specify the version of the compiler.
    /// Defaults to the latest stable compiler version (at least 2)
    #[clap(long, value_parser = clap::value_parser!(CompilerVersion),
           default_value = LATEST_STABLE_COMPILER_VERSION,)]
    pub compiler_version: Option<CompilerVersion>,

    /// Specify the language version to be supported.
    /// Defaults to the latest stable language version (at least 2)
    #[clap(long, value_parser = clap::value_parser!(LanguageVersion),
           default_value = LATEST_STABLE_LANGUAGE_VERSION,)]
    pub language_version: Option<LanguageVersion>,
}

impl CompileScriptFunction {
    pub fn compile(
        &self,
        w: &DiagWriter,
        script_name: &str,
        prompt_options: PromptOptions,
    ) -> CliTypedResult<(Vec<u8>, HashValue)> {
        if let Some(compiled_script_path) = &self.compiled_script_path {
            let bytes = std::fs::read(compiled_script_path).map_err(|e| {
                CliError::IO(format!("Unable to read {:?}", self.compiled_script_path), e)
            })?;
            let hash = HashValue::sha3_256_of(bytes.as_slice());
            return Ok((bytes, hash));
        }

        // Check script file
        let script_path = self
            .script_path
            .as_ref()
            .ok_or_else(|| {
                CliError::CommandArgumentError(
                    "Must choose either --compiled-script-path or --script-path".to_string(),
                )
            })?
            .as_path();
        if !script_path.exists() {
            return Err(CliError::CommandArgumentError(format!(
                "{} does not exist",
                script_path.display()
            )));
        } else if script_path.is_dir() {
            return Err(CliError::CommandArgumentError(format!(
                "{} is a directory",
                script_path.display()
            )));
        }

        // Compile script
        compile_in_temp_dir(
            w,
            script_name,
            script_path,
            &self.framework_package_args,
            prompt_options,
            self.bytecode_version,
            self.language_version
                .or_else(|| Some(LanguageVersion::latest_stable())),
            self.compiler_version
                .or_else(|| Some(CompilerVersion::latest_stable())),
        )
    }
}

pub fn compile_in_temp_dir(
    w: &DiagWriter,
    script_name: &str,
    script_path: &Path,
    framework_package_args: &FrameworkPackageArgs,
    prompt_options: PromptOptions,
    bytecode_version: Option<u32>,
    language_version: Option<LanguageVersion>,
    compiler_version: Option<CompilerVersion>,
) -> CliTypedResult<(Vec<u8>, HashValue)> {
    // Make a temporary directory for compilation
    let temp_dir = TempDir::new().map_err(|err| {
        CliError::UnexpectedError(format!("Failed to create temporary directory {}", err))
    })?;

    // Initialize a move directory
    let package_dir = temp_dir.path();
    framework_package_args.init_move_dir(
        package_dir,
        script_name,
        BTreeMap::new(),
        prompt_options,
    )?;

    // Insert the new script
    let sources_dir = package_dir.join("sources");
    let new_script_path = if let Some(file_name) = script_path.file_name() {
        sources_dir.join(file_name)
    } else {
        // If for some reason we can't get the move file
        sources_dir.join("script.move")
    };
    fs::copy(script_path, new_script_path.as_path()).map_err(|err| {
        CliError::IO(
            format!(
                "Failed to copy {} to {}",
                script_path.display(),
                new_script_path.display()
            ),
            err,
        )
    })?;

    // Compile the script
    compile_script(
        w,
        framework_package_args.skip_fetch_latest_git_deps,
        package_dir,
        bytecode_version,
        language_version,
        compiler_version,
    )
}

fn compile_script(
    w: &DiagWriter,
    skip_fetch_latest_git_deps: bool,
    package_dir: &Path,
    bytecode_version: Option<u32>,
    language_version: Option<LanguageVersion>,
    compiler_version: Option<CompilerVersion>,
) -> CliTypedResult<(Vec<u8>, HashValue)> {
    let build_options = BuildOptions {
        with_srcs: false,
        with_abis: false,
        with_source_maps: false,
        with_error_map: false,
        skip_fetch_latest_git_deps,
        bytecode_version,
        language_version,
        compiler_version,
        ..BuildOptions::default()
    };

    let pack = BuiltPackage::build_to(w, package_dir.to_path_buf(), build_options)
        .map_err(|e| CliError::MoveCompilationError(format!("{:#}", e)))?;

    let scripts_count = pack.script_count();

    if scripts_count != 1 {
        return Err(CliError::UnexpectedError(format!(
            "Only one script can be prepared a time. Make sure one and only one script file \
                is included in the Move package. Found {} scripts.",
            scripts_count
        )));
    }

    let bytes = pack.extract_script_code().pop().unwrap();
    let hash = HashValue::sha3_256_of(bytes.as_slice());
    Ok((bytes, hash))
}
