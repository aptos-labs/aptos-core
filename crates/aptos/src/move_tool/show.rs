// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::IncludedArtifactsArgs;
use crate::common::types::{CliCommand, CliError, CliResult, CliTypedResult, MovePackageOptions};
use anyhow::Context;
use aptos_api_types::{Address, Bytecode, IdentifierWrapper, MoveFunction, MoveFunctionVisibility};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_types::transaction::EntryABI;
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use move_core_types::{account_address::AccountAddress, identifier::Identifier};
use serde::Serialize;

#[derive(Subcommand)]
pub enum ShowTool {
    Abi(ShowAbi),
    Abiv2(ShowAbiV2),
}

impl ShowTool {
    pub async fn execute_serialized(self) -> CliResult {
        match self {
            Self::Abi(tool) => tool.execute_serialized().await,
            Self::Abiv2(tool) => tool.execute_serialized().await,
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

    #[clap(flatten)]
    included_artifacts_args: IncludedArtifactsArgs,

    #[clap(flatten)]
    move_options: MovePackageOptions,
}

#[async_trait]
impl CliCommand<Vec<EntryABI>> for ShowAbi {
    fn command_name(&self) -> &'static str {
        "ShowAbi"
    }

    async fn execute(self) -> CliTypedResult<Vec<EntryABI>> {
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

        Ok(abis)
    }
}

/// Compile the package and show information about the ABI (v2 format) of the compiled modules.
///
/// This differs from `aptos move show abi` in that it derives function information directly from
/// compiled modules so it can include:
/// - public functions
/// - entry functions (even if not public)
/// - view functions (with an explicit `is_view` field)
///
/// For example, this would show the function `transfer` in the module `coin`:
///
/// aptos move show abiv2 --modules coin --names transfer
#[derive(Parser)]
pub struct ShowAbiV2 {
    /// If provided, only show items from the given Move modules. These should be module
    /// names, not file paths. For example, `coin`.
    #[clap(long, num_args = 0..)]
    modules: Vec<String>,

    /// If provided, only show items with the given names. For example, `transfer`.
    #[clap(long, num_args = 0..)]
    names: Vec<String>,

    #[clap(flatten)]
    included_artifacts_args: IncludedArtifactsArgs,

    #[clap(flatten)]
    move_options: MovePackageOptions,
}

/// Simplified view of a module ABI for CLI output.
#[derive(Clone, Debug, Serialize)]
pub struct ShowAbiModule {
    pub address: Address,
    pub name: IdentifierWrapper,
    pub functions: Vec<MoveFunction>,
}

#[async_trait]
impl CliCommand<Vec<ShowAbiModule>> for ShowAbiV2 {
    fn command_name(&self) -> &'static str {
        "ShowAbiV2"
    }

    async fn execute(self) -> CliTypedResult<Vec<ShowAbiModule>> {
        let build_options = BuildOptions {
            install_dir: self.move_options.output_dir.clone(),
            // Not required for v2 output, but harmless if other code wants it.
            with_abis: false,
            ..self
                .included_artifacts_args
                .included_artifacts
                .build_options(&self.move_options)?
        };
        let package = BuiltPackage::build(self.move_options.get_package_path()?, build_options)
            .map_err(|e| CliError::MoveCompilationError(format!("{:#}", e)))?;

        let mut result: Vec<ShowAbiModule> = vec![];
        for module in package.modules() {
            let (address, module_name) = <(AccountAddress, Identifier)>::from(module.self_id());

            // Module filter (accepts module names only, e.g. "coin").
            if !self.modules.is_empty() && !self.modules.contains(&module_name.to_string()) {
                continue;
            }

            let mut functions: Vec<MoveFunction> = module
                .function_defs
                .iter()
                .map(|def| module.new_move_function(def))
                .filter(|f| {
                    f.is_entry
                        || matches!(f.visibility, MoveFunctionVisibility::Public)
                        || f.is_view
                })
                .collect();

            // Name filter (function names only, e.g. "transfer").
            if !self.names.is_empty() {
                functions.retain(|f| self.names.iter().any(|n| n == f.name.as_str()));
            }

            if functions.is_empty() {
                continue;
            }

            result.push(ShowAbiModule {
                address: address.into(),
                name: module_name.into(),
                functions,
            });
        }

        Ok(result)
    }
}
