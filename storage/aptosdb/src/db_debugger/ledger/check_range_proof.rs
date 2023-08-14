// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{db_debugger::common::DbDir, ledger_store::LedgerStore};
use aptos_crypto::hash::CryptoHash;
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use aptos_types::transaction::Version;
use clap::Parser;
use std::sync::Arc;

#[derive(Parser)]
#[clap(
    about = "Check the accumulator by verifying a range proof against all LedgerInfos newer than it."
)]
pub struct Cmd {
    #[clap(flatten)]
    db_dir: DbDir,

    start_version: Version,

    num_versions: usize,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        let ledger_db = Arc::new(self.db_dir.open_ledger_db()?);
        let store = LedgerStore::new(ledger_db);
        let ledger_info = store.get_latest_ledger_info()?;
        println!("Latest LedgerInfo: {:?}", ledger_info);

        println!("Checking Range proof...");

        let txn_infos: Vec<_> = store
            .get_transaction_info_iter(self.start_version, self.num_versions)?
            .collect::<Result<_>>()?;
        ensure!(
            txn_infos.len() == self.num_versions,
            "expecting {} txns, got {}",
            self.num_versions,
            txn_infos.len(),
        );
        let txn_info_hashes: Vec<_> = txn_infos.iter().map(CryptoHash::hash).collect();

        let last_version = self.start_version + self.num_versions as u64 - 1;
        let last_version_epoch = store.get_epoch(last_version)?;
        for epoch in last_version_epoch..=ledger_info.ledger_info().epoch() {
            println!("Check against epoch {} LedgerInfo.", epoch);
            let li = store.get_latest_ledger_info_in_epoch(epoch)?;
            println!(
                "    Root hash: {:?}",
                li.ledger_info().transaction_accumulator_hash()
            );
            let range_proof = store.get_transaction_range_proof(
                Some(self.start_version),
                self.num_versions as u64,
                li.ledger_info().version(),
            )?;
            range_proof.verify(
                li.ledger_info().transaction_accumulator_hash(),
                Some(self.start_version),
                &txn_info_hashes,
            )?;
        }

        println!("Done.");
        Ok(())
    }
}
