// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod common;
mod state_tree;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub enum Cmd {
    #[clap(subcommand)]
    StateTree(state_tree::Cmd),
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        match self {
            Cmd::StateTree(cmd) => cmd.run(),
        }
    }
}
