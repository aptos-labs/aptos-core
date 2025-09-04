// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db_debugger::common::DbDir, schema::transaction_accumulator::TransactionAccumulatorSchema,
};
use aptos_crypto::hash::CryptoHash;
use aptos_storage_interface::{AptosDbError, Result, db_ensure as ensure};
use aptos_types::{proof::position::Position, transaction::Version};
use clap::Parser;
use std::sync::Arc;

#[derive(Parser)]
#[clap(about = "Check TransactionInfo hashes match the state tree leaves.")]
pub struct Cmd {
    #[clap(flatten)]
    db_dir: DbDir,

    start_version: Version,

    num_versions: usize,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        let ledger_db = Arc::new(self.db_dir.open_ledger_db()?);
        println!(
            "Latest LedgerInfo: {:?}",
            ledger_db.metadata_db().get_latest_ledger_info_option()
        );

        println!("Checking that TransactionInfo hashes matches accumulator leaf hashes...");
        let txn_info_iter = ledger_db
            .transaction_info_db()
            .get_transaction_info_iter(self.start_version, self.num_versions)?;
        let mut version = self.start_version;
        for res in txn_info_iter {
            let txn_info = res?;
            let leaf_hash =
                ledger_db
                    .transaction_accumulator_db_raw()
                    .get::<TransactionAccumulatorSchema>(&Position::from_leaf_index(version))?;
            let txn_info_hash = txn_info.hash();

            ensure!(
                leaf_hash.as_ref() == Some(&txn_info_hash),
                "Found mismatch: version: {}, txn_info_hash: {:?}, leaf_hash: {:?}",
                version,
                txn_info_hash,
                leaf_hash,
            );

            if version % 10_000 == 0 {
                println!("Good until version {}.", version);
            }

            version += 1;
        }
        ensure!(
            version - self.start_version == self.num_versions as u64,
            "Didn't see all versions requested, missing {}",
            version,
        );
        println!("Done.");

        Ok(())
    }
}
