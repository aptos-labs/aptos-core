// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{aptos::AptosUpdateTool, revela::RevelaUpdateTool};
#[cfg(feature = "prover-deps")]
use crate::update::prover_dependencies::ProverDependencyInstaller;
use crate::{
    common::types::{CliCommand, CliResult},
    update::{move_mutation_test::MutationTestUpdaterTool, movefmt::FormatterUpdateTool},
};
use clap::Subcommand;

/// Update the CLI or other tools it depends on.
#[derive(Subcommand)]
pub enum UpdateTool {
    Aptos(AptosUpdateTool),
    Revela(RevelaUpdateTool),
    Movefmt(FormatterUpdateTool),
    MoveMutationTest(MutationTestUpdaterTool),
    #[cfg(feature = "prover-deps")]
    ProverDependencies(ProverDependencyInstaller),
}

impl UpdateTool {
    pub async fn execute(self) -> CliResult {
        match self {
            UpdateTool::Aptos(tool) => tool.execute_serialized().await,
            UpdateTool::Revela(tool) => tool.execute_serialized().await,
            UpdateTool::Movefmt(tool) => tool.execute_serialized().await,
            UpdateTool::MoveMutationTest(tool) => tool.execute_serialized().await,
            #[cfg(feature = "prover-deps")]
            UpdateTool::ProverDependencies(tool) => tool.execute_serialized().await,
        }
    }
}
