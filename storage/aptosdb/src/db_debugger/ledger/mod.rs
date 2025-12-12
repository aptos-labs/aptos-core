// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod check_range_proof;
mod check_txn_info_hashes;

use aptos_storage_interface::Result;

#[derive(clap::Subcommand)]
#[clap(about = "Check the ledger.")]
pub enum Cmd {
    CheckTransactionInfoHashes(check_txn_info_hashes::Cmd),
    CheckRangeProof(check_range_proof::Cmd),
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        match self {
            Self::CheckTransactionInfoHashes(cmd) => cmd.run(),
            Self::CheckRangeProof(cmd) => cmd.run(),
        }
    }
}
