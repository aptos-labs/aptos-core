// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod opened;

use velor_storage_interface::Result;

#[derive(clap::Subcommand)]
#[clap(about = "Examine databases.")]
pub enum Cmd {
    Opened(opened::Cmd),
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Opened(cmd) => cmd.run(),
        }
    }
}
