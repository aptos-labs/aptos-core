// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod print_db_versions;

use aptos_storage_interface::Result;

#[derive(clap::Subcommand)]
#[clap(about = "Examine databases.")]
pub enum Cmd {
    PrintDbVersions(print_db_versions::Cmd),
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        match self {
            Self::PrintDbVersions(cmd) => cmd.run(),
        }
    }
}
