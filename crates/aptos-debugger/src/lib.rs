// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub enum Cmd {
    #[clap(subcommand)]
    AptosDb(aptos_db_tool::DBTool),

    Decode(aptos_move_debugger::bcs_txn_decoder::Command),

    DumpPendingTxns(aptos_consensus::util::db_tool::Command),

    #[clap(subcommand)]
    Move(aptos_move_debugger::common::Command),
}

impl Cmd {
    pub async fn run(self) -> Result<()> {
        match self {
            Cmd::AptosDb(cmd) => cmd.run().await,
            Cmd::Decode(cmd) => cmd.run().await,
            Cmd::DumpPendingTxns(cmd) => cmd.run().await,
            Cmd::Move(cmd) => cmd.run().await,
        }
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Cmd::command().debug_assert()
}
