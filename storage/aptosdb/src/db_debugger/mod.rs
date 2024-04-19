// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod checkpoint;
mod common;
mod examine;
pub mod ledger;
pub mod state_tree;
pub mod truncate;

use aptos_storage_interface::Result;
use clap::Parser;

#[derive(Parser, Clone)]
pub struct ShardingConfig {
    #[clap(long)]
    enable_storage_sharding: bool,
}

#[derive(Parser)]
pub enum Cmd {
    #[clap(subcommand)]
    StateTree(state_tree::Cmd),

    Checkpoint(checkpoint::Cmd),

    #[clap(subcommand)]
    Ledger(ledger::Cmd),

    Truncate(truncate::Cmd),

    #[clap(subcommand)]
    Examine(examine::Cmd),
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        match self {
            Cmd::StateTree(cmd) => cmd.run(),
            Cmd::Checkpoint(cmd) => cmd.run(),
            Cmd::Ledger(cmd) => cmd.run(),
            Cmd::Truncate(cmd) => cmd.run(),
            Cmd::Examine(cmd) => cmd.run(),
        }
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Cmd::command().debug_assert()
}
