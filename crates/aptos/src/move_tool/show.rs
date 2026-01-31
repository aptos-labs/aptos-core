// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::IncludedArtifactsArgs;
use crate::common::{
    types::{CliCommand, CliError, CliResult, CliTypedResult, MovePackageOptions},
    utils::read_from_file,
};
use anyhow::Context;
use aptos_api_types::{MoveModule, MoveModuleBytecode};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_types::transaction::EntryABI;
use async_trait::async_trait;
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::path::PathBuf;

/// The format for displaying ABIs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum AbiFormat {
    /// The default ABI format.
    #[default]
    Default,
    /// The Aptos REST API JSON ABI format.
    AptosRest,
}

/// The output of the `show abi` command, which varies based on the format.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ShowAbiOutput {
    /// The default ABI format, returning a list of entry ABIs.
    Default(Vec<EntryABI>),
    /// The Aptos REST API JSON ABI format, returning the module ABI.
    AptosRest(Option<MoveModule>),
}

#[derive(Subcommand)]
pub enum ShowTool {
    Abi(ShowAbi),
}

impl ShowTool {
    pub async fn execute_serialized(self) -> CliResult {
        match self {
            Self::Abi(tool) => tool.execute_serialized().await,
        }
    }
}

/// Compile the package and show information about the ABIs of the compiled modules.
///
/// For example, this would show the function `transfer` in the module `coin`:
///
/// aptos move show abi --modules coin --names transfer
///
#[derive(Parser)]
pub struct ShowAbi {
    /// If provided, only show items from the given Move modules. These should be module
    /// names, not file paths. For example, `coin`.
    #[clap(long, num_args = 0..)]
    modules: Vec<String>,

    /// If provided, only show items with the given names. For example, `transfer`.
    #[clap(long, num_args = 0..)]
    names: Vec<String>,

    /// The path to a compiled bytecode file to show the ABIs for. For example, `coin.mv`.
    /// This can only be used with the `--format` option set to `aptos-rest`.
    #[clap(long, value_parser)]
    bytecode_path: Option<PathBuf>,

    /// The format to display the ABIs in.
    #[clap(long, value_enum, default_value_t = AbiFormat::Default)]
    format: AbiFormat,

    #[clap(flatten)]
    included_artifacts_args: IncludedArtifactsArgs,

    #[clap(flatten)]
    move_options: MovePackageOptions,
}

#[async_trait]
impl CliCommand<ShowAbiOutput> for ShowAbi {
    fn command_name(&self) -> &'static str {
        "ShowAbi"
    }

    async fn execute(self) -> CliTypedResult<ShowAbiOutput> {
        if self.format == AbiFormat::AptosRest {
            if self.bytecode_path.is_none() {
                return Err(CliError::CommandArgumentError(
                    "The --bytecode-path option is required when --format is set to aptos-rest"
                        .to_string(),
                ));
            }

            let bytecode = read_from_file(self.bytecode_path.as_ref().unwrap())?;
            let abi = MoveModuleBytecode::new(bytecode)
                .try_parse_abi()
                .map_err(|e| CliError::UnexpectedError(format!("{:#}", e)))?
                .abi;
            return Ok(ShowAbiOutput::AptosRest(abi));
        }

        if self.bytecode_path.is_some() && self.format == AbiFormat::Default {
            return Err(CliError::CommandArgumentError(
                "The --bytecode-path option is only allowed when --format is set to aptos-rest"
                    .to_string(),
            ));
        }

        let build_options = BuildOptions {
            install_dir: self.move_options.output_dir.clone(),
            with_abis: true,
            ..self
                .included_artifacts_args
                .included_artifacts
                .build_options(&self.move_options)?
        };
        // Build the package.
        let package = BuiltPackage::build(self.move_options.get_package_path()?, build_options)
            .map_err(|e| CliError::MoveCompilationError(format!("{:#}", e)))?;

        // Get ABIs from the package.
        let abis = package
            .extract_abis()
            .context("No ABIs found after compilation")?;

        // Filter the ABIs based on the filters passed in.
        let abis = abis
            .into_iter()
            .filter(|abi| {
                let name = abi.name().to_string();
                if !self.names.is_empty() && !self.names.contains(&name) {
                    return false;
                }
                match &abi {
                    EntryABI::EntryFunction(func) => {
                        if !self.modules.is_empty()
                            && !self
                                .modules
                                .contains(&func.module_name().name().to_string())
                        {
                            return false;
                        }
                    },
                    EntryABI::TransactionScript(_) => {
                        // If there were any modules specified we ignore scripts.
                        if !self.modules.is_empty() {
                            return false;
                        }
                    },
                }
                true
            })
            .collect();

        Ok(ShowAbiOutput::Default(abis))
    }
}
