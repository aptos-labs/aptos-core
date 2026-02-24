// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod check_stale_nodes;
mod get_leaf;
mod get_path;
mod get_snapshots;

use aptos_storage_interface::Result;

#[derive(clap::Subcommand)]
pub enum Cmd {
    GetSnapshots(get_snapshots::Cmd),
    GetPath(get_path::Cmd),
    GetLeaf(get_leaf::Cmd),
    CheckStaleNodes(check_stale_nodes::Cmd),
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        match self {
            Self::GetSnapshots(cmd) => Ok(cmd.run()?),
            Self::GetPath(cmd) => cmd.run(),
            Self::GetLeaf(cmd) => cmd.run(),
            Self::CheckStaleNodes(cmd) => cmd.run(),
        }
    }
}
