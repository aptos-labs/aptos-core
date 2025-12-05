// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod opened;

use aptos_storage_interface::Result;

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
