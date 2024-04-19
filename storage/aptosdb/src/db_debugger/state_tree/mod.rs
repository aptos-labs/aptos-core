// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod get_path;
mod get_snapshots;

use aptos_storage_interface::Result;

/// Tool supports listing snapshots before version and printing node in merkel tree with version and nibble path
#[derive(clap::Subcommand)]
pub enum Cmd {
    GetSnapshots(get_snapshots::Cmd),
    GetPath(get_path::Cmd),
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        match self {
            Self::GetSnapshots(cmd) => Ok(cmd.run()?),
            Self::GetPath(cmd) => cmd.run(),
        }
    }
}
