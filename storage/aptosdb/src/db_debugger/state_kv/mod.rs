// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

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
