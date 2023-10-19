// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod consensus_db;
mod print_db_versions;

use anyhow::Result;

#[derive(clap::Subcommand)]
#[clap(about = "Examine databases.")]
pub enum Cmd {
    PrintDbVersions(print_db_versions::Cmd),
    ConsensusDb(consensus_db::Cmd),
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        match self {
            Self::PrintDbVersions(cmd) => cmd.run(),
            _ => unreachable!(),
            //Self::ConsensusDb(cmd) => cmd.run(),
        }
    }
}
