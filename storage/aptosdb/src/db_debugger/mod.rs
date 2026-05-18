// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod checkpoint;
mod common;
mod examine;
pub mod ledger;
pub mod state_kv;
pub mod state_tree;
pub mod truncate;
pub mod validation;
mod watch;

use aptos_storage_interface::Result;
use clap::Parser;

#[derive(Parser)]
pub enum Cmd {
    #[clap(subcommand)]
    StateTree(state_tree::Cmd),

    #[clap(subcommand)]
    StateKv(state_kv::Cmd),

    Checkpoint(checkpoint::Cmd),

    #[clap(subcommand)]
    Ledger(ledger::Cmd),

    Truncate(truncate::Cmd),

    #[clap(subcommand)]
    Examine(examine::Cmd),

    #[clap(subcommand)]
    IndexerValidation(validation::Cmd),

    #[clap(subcommand)]
    Watch(watch::Cmd),
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        match self {
            Cmd::StateTree(cmd) => cmd.run(),
            Cmd::StateKv(cmd) => cmd.run(),
            Cmd::Checkpoint(cmd) => cmd.run(),
            Cmd::Ledger(cmd) => cmd.run(),
            Cmd::Truncate(cmd) => cmd.run(),
            Cmd::Examine(cmd) => cmd.run(),
            Cmd::IndexerValidation(cmd) => cmd.run(),
            Cmd::Watch(cmd) => cmd.run(),
        }
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Cmd::command().debug_assert()
}
