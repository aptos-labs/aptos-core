// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod get_value;
mod scan_snapshot;

use aptos_storage_interface::Result;

#[derive(clap::Subcommand)]
pub enum Cmd {
    GetValue(get_value::Cmd),
    ScanSnapshot(scan_snapshot::Cmd),
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        match self {
            Self::GetValue(cmd) => cmd.run(),
            Self::ScanSnapshot(cmd) => cmd.run(),
        }
    }
}
