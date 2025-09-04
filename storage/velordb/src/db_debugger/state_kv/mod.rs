// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod get_value;
mod scan_snapshot;

use velor_storage_interface::Result;

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
