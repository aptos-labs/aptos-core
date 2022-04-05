// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A tool for interacting with Move
//!
//! TODO: Examples
//!

use crate::CliResult;
use clap::{ArgEnum, Parser, Subcommand};
use move_core_types::errmap::ErrorMapping;
use move_vm_types::gas_schedule::INITIAL_COST_SCHEDULE;

pub mod chain;

/// CLI tool for performing Move tasks
///
#[derive(ArgEnum, Subcommand)]
pub enum MoveTool {
    Command(MoveCli),
}

impl MoveTool {
    pub async fn execute(self) -> CliResult {
        match self {
            // TODO: Rethink using the Move CLI and think about how we can make the experience better
            MoveTool::Command(tool) => tool.execute(),
        }
    }
}

#[derive(Parser)]
pub struct MoveCli {
    #[clap(flatten)]
    move_args: move_cli::Move,
    #[clap(subcommand)]
    command: move_cli::Command,
}

impl MoveCli {
    fn execute(self) -> CliResult {
        let error_descriptions: ErrorMapping =
            bcs::from_bytes(cached_framework_packages::error_map())
                .map_err(|err| err.to_string())?;
        move_cli::run_cli(
            aptos_vm::natives::aptos_natives(),
            &INITIAL_COST_SCHEDULE,
            &error_descriptions,
            &self.move_args,
            &self.command,
        )
        .map(|_| "".to_string())
        .map_err(|err| err.to_string())
    }
}
