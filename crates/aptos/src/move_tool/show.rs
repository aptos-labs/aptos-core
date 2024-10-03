// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::IncludedArtifactsArgs;
use crate::common::types::{CliCommand, CliError, CliResult, CliTypedResult, MovePackageDir};
use anyhow::Context;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_types::transaction::EntryABI;
use async_trait::async_trait;
use clap::{Parser, Subcommand};

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

    #[clap(flatten)]
    included_artifacts_args: IncludedArtifactsArgs,

    #[clap(flatten)]
    move_options: MovePackageDir,
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
