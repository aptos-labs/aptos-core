// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

extern crate core;

mod backup;
mod backup_maintenance;
mod debugger;
mod replay_verify;
pub mod restore;
#[cfg(test)]
mod tests;
mod utils;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[clap(name = "Aptos db tool", author, disable_version_flag = true)]
pub enum DBTool {
    #[clap(subcommand)]
    Backup(backup::Command),
    #[clap(subcommand)]
    Restore(restore::Command),
    ReplayVerify(replay_verify::Opt),
    #[clap(subcommand)]
    Debug(debugger::Command),
    #[clap(subcommand)]
    BackupMaintenance(backup_maintenance::Command),
}

impl DBTool {
    pub async fn run(self) -> Result<()> {
        match self {
            DBTool::Backup(cmd) => cmd.run().await,
            DBTool::Restore(cmd) => cmd.run().await,
            DBTool::ReplayVerify(cmd) => cmd.run().await,
            DBTool::BackupMaintenance(cmd) => cmd.run().await,
            DBTool::Debug(cmd) => cmd.run(),
        }
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    DBTool::command().debug_assert()
}
