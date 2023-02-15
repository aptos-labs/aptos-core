// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_db::db_debugger::{checkpoint, ledger, state_tree, truncate};
use clap::Parser;

/// List snapshots, print nodes, make DB checkpoints and validate ledger hash
#[derive(Parser)]
pub enum Command {
    #[clap(subcommand)]
    StateTree(state_tree::Cmd),
    Checkpoint(checkpoint::Cmd),
    #[clap(subcommand)]
    Ledger(ledger::Cmd),
    Truncate(truncate::Cmd),
}

impl Command {
    pub fn run(self) -> Result<()> {
        match self {
            Command::StateTree(cmd) => cmd.run(),
            Command::Checkpoint(cmd) => cmd.run(),
            Command::Ledger(cmd) => cmd.run(),
            Command::Truncate(cmd) => cmd.run(),
        }
    }
}
