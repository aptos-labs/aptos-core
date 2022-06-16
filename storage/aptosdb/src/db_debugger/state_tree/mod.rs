// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod get_path;
mod get_snapshots;

use anyhow::Result;

#[derive(clap::Subcommand)]
pub enum Cmd {
    GetSnapshots(get_snapshots::Cmd),
    GetPath(get_path::Cmd),
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        match self {
            Self::GetSnapshots(cmd) => cmd.run(),
            Self::GetPath(cmd) => cmd.run(),
        }
    }
}
