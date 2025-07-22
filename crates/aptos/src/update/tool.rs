// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// use super::{aptos::AptosUpdateTool, revela::RevelaUpdateTool};
use crate::{
    common::types::{CliCommand, CliResult},
    update::movefmt::FormatterUpdateTool,
};
use clap::Subcommand;

/// Update the CLI or other tools it depends on.
#[derive(Subcommand)]
pub enum UpdateTool {
    // Aptos(AptosUpdateTool),
    // Revela(RevelaUpdateTool),
    Movefmt(FormatterUpdateTool),
}

impl UpdateTool {
    pub async fn execute(self) -> CliResult {
        match self {
            // UpdateTool::Aptos(tool) => tool.execute_serialized().await,
            // UpdateTool::Revela(tool) => tool.execute_serialized().await,
            UpdateTool::Movefmt(tool) => tool.execute_serialized().await,
        }
    }
}
