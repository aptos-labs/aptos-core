// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::DIEM_TRANSACTION_PRUNER_LEAST_READABLE_VERSION, pruner::db_pruner::DBPruner,
    transaction::TransactionSchema, transaction_accumulator::TransactionAccumulatorSchema,
    transaction_by_account::TransactionByAccountSchema,
    transaction_by_hash::TransactionByHashSchema, transaction_info::TransactionInfoSchema,
};
use aptos_crypto::hash::CryptoHash;
use aptos_logger::{error, info};
use aptos_types::{
    proof::position::Position,
    transaction::{AtomicVersion, Transaction, Version},
};
use schemadb::{ReadOptions, SchemaBatch, DB};
use std::{
    cmp::min,
    sync::{atomic::Ordering, Arc},
    thread::sleep,
    time::Duration,
};

pub struct TransactionStorePruner {
    db: Arc<DB>,
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    least_readable_version: AtomicVersion,
}

impl DBPruner for TransactionStorePruner {
    fn initialize(&self) {
        loop {
            match self.initialize_least_readable_version() {
                Ok(least_readable_version) => {
                    info!(
                        least_readable_version = least_readable_version,
                        "[transaction pruner] initialized."
                    );
                    self.record_progress(least_readable_version);
                    return;
                }
                Err(e) => {
                    error!(
                        error = ?e,
                        "[transaction pruner] Error on first seek. Retrying in 1 second.",
                    );
                    sleep(Duration::from_secs(1));
                }
            }
        }
    }

    fn prune(&self, max_versions: usize) -> anyhow::Result<Version> {
        let least_readable_version = self.least_readable_version();
        // Current target version  might be less than the target version to ensure we don't prune
        // more than max_version in one go.
        let current_target_version = min(
            least_readable_version + max_versions as u64,
            self.target_version(),
        );
        let candidate_transactions = self
            .get_pruning_candidate_transactions(least_readable_version, current_target_version)?;
        let mut db_batch = SchemaBatch::new();
        self.prune_transaction_by_hash(&candidate_transactions, &mut db_batch)?;
        self.prune_transaction_schema(current_target_version, &mut db_batch)?;
        self.prune_transaction_by_account(&candidate_transactions, &mut db_batch)?;
        self.prune_transaction_accumulator(current_target_version, &mut db_batch)?;
        self.db.write_schemas(db_batch)?;

        self.record_progress(current_target_version);
        Ok(current_target_version)
    }

    fn initialize_least_readable_version(&self) -> anyhow::Result<Version> {
        let mut iter = self.db.iter::<TransactionSchema>(ReadOptions::default())?;
        iter.seek_to_first();
        let version = iter.next().transpose()?.map_or(0, |(version, _)| version);
        Ok(version)
    }

    fn least_readable_version(&self) -> Version {
        self.least_readable_version.load(Ordering::Relaxed)
    }

    fn set_target_version(&self, target_version: Version) {
        self.target_version.store(target_version, Ordering::Relaxed)
    }

    fn target_version(&self) -> Version {
        self.target_version.load(Ordering::Relaxed)
    }

    fn record_progress(&self, least_readable_version: Version) {
        self.least_readable_version
            .store(least_readable_version, Ordering::Relaxed);
        DIEM_TRANSACTION_PRUNER_LEAST_READABLE_VERSION.set(least_readable_version as i64);
    }

    fn is_pruning_pending(&self) -> bool {
        self.least_readable_version() >= self.target_version()
    }
}

impl TransactionStorePruner {
    pub fn new(db: Arc<DB>) -> Self {
        TransactionStorePruner {
            db,
            target_version: AtomicVersion::new(0),
            least_readable_version: AtomicVersion::new(0),
        }
    }

    fn get_pruning_candidate_transactions(
        &self,
        start: Version,
        end: Version,
    ) -> anyhow::Result<Vec<Transaction>> {
        let mut transactions: Vec<Transaction> = vec![];
        let mut iter = self.db.iter::<TransactionSchema>(ReadOptions::default())?;
        iter.seek(&start)?;
        for x in iter {
            let (version, transaction) = x?;
            if version == end {
                break;
            }
            transactions.push(transaction);
        }
        Ok(transactions)
    }

    fn prune_transaction_by_hash(
        &self,
        transactions: &[Transaction],
        db_batch: &mut SchemaBatch,
    ) -> anyhow::Result<()> {
        for transaction in transactions {
            db_batch.delete::<TransactionByHashSchema>(&transaction.hash())?;
        }
        Ok(())
    }

    fn prune_transaction_by_account(
        &self,
        transactions: &[Transaction],
        db_batch: &mut SchemaBatch,
    ) -> anyhow::Result<()> {
        for transaction in transactions {
            if let Transaction::UserTransaction(txn) = transaction {
                db_batch
                    .delete::<TransactionByAccountSchema>(&(txn.sender(), txn.sequence_number()))?;
            }
        }
        Ok(())
    }

    fn prune_transaction_schema(
        &self,
        current_target_version: Version,
        db_batch: &mut SchemaBatch,
    ) -> anyhow::Result<()> {
        db_batch.delete_range::<TransactionSchema>(
            &self.least_readable_version(),
            &current_target_version,
        )?;
        db_batch.delete_range::<TransactionInfoSchema>(
            &self.least_readable_version(),
            &current_target_version,
        )?;
        Ok(())
    }

    fn prune_transaction_accumulator(
        &self,
        current_target_version: Version,
        db_batch: &mut SchemaBatch,
    ) -> anyhow::Result<()> {
        let least_readable_position = if self.least_readable_version() > 0 {
            let x = Position::root_from_leaf_index(self.least_readable_version());
            x.left_child()
        } else {
            // Handle this as a special case when least_readable_version is 0
            Position::root_from_leaf_index(self.least_readable_version())
        };
        let target_position = Position::root_from_leaf_index(current_target_version).left_child();

        db_batch.delete_range::<TransactionAccumulatorSchema>(
            &least_readable_position,
            &target_position,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod test;
