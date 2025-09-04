// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::OTHER_TIMERS_SECONDS,
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        transaction::TransactionSchema,
        transaction_by_hash::TransactionByHashSchema,
        transaction_summaries_by_account::TransactionSummariesByAccountSchema,
    },
    utils::iterators::ExpectContinuousVersions,
};
use aptos_crypto::hash::{CryptoHash, HashValue};
use aptos_db_indexer_schemas::schema::ordered_transaction_by_account::OrderedTransactionByAccountSchema;
use aptos_metrics_core::TimerHelper;
use aptos_schemadb::{
    DB,
    batch::{NativeBatch, SchemaBatch, WriteBatch},
};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::transaction::{IndexedTransactionSummary, ReplayProtector, Transaction, Version};
use rayon::prelude::*;
use std::{path::Path, sync::Arc};

#[derive(Debug)]
pub(crate) struct TransactionDb {
    db: Arc<DB>,
}

impl TransactionDb {
    pub(super) fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    pub(super) fn create_checkpoint(&self, path: impl AsRef<Path>) -> Result<()> {
        self.db.create_checkpoint(path)
    }

    pub(super) fn write_pruner_progress(&self, version: Version) -> Result<()> {
        self.db.put::<DbMetadataSchema>(
            &DbMetadataKey::TransactionPrunerProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub(crate) fn db(&self) -> &DB {
        &self.db
    }

    pub(crate) fn write_schemas(&self, batch: SchemaBatch) -> Result<()> {
        self.db.write_schemas(batch)
    }

    /// Returns signed transaction given its `version`.
    pub(crate) fn get_transaction(&self, version: Version) -> Result<Transaction> {
        self.db
            .get::<TransactionSchema>(&version)?
            .ok_or_else(|| AptosDbError::NotFound(format!("Txn {version}")))
    }

    /// Returns an iterator that yields at most `num_transactions` transactions starting from `start_version`.
    pub(crate) fn get_transaction_iter(
        &self,
        start_version: Version,
        num_transactions: usize,
    ) -> Result<impl Iterator<Item = Result<Transaction>> + '_> {
        let mut iter = self.db.iter::<TransactionSchema>()?;
        iter.seek(&start_version)?;
        iter.expect_continuous_versions(start_version, num_transactions)
    }

    /// Returns the version of a transaction given its hash.
    pub(crate) fn get_transaction_version_by_hash(
        &self,
        hash: &HashValue,
        ledger_version: Version,
    ) -> Result<Option<Version>> {
        Ok(match self.db.get::<TransactionByHashSchema>(hash)? {
            Some(version) if version <= ledger_version => Some(version),
            _ => None,
        })
    }

    pub(crate) fn commit_transactions(
        &self,
        first_version: Version,
        transactions: &[Transaction],
        skip_index: bool,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_transactions"]);
        let chunk_size = transactions.len() / 4 + 1;
        let batches = transactions
            .par_chunks(chunk_size)
            .enumerate()
            .map(|(chunk_index, txns_in_chunk)| -> Result<NativeBatch> {
                let mut batch = self.db().new_native_batch();
                let chunk_first_version = first_version + (chunk_size * chunk_index) as u64;
                txns_in_chunk
                    .iter()
                    .enumerate()
                    .try_for_each(|(i, txn)| -> Result<()> {
                        self.put_transaction(
                            chunk_first_version + i as u64,
                            txn,
                            skip_index,
                            &mut batch,
                        )?;

                        Ok(())
                    })?;
                Ok(batch)
            })
            .collect::<Result<Vec<_>>>()?;

        // Commit batches one by one for now because committing them in parallel will cause gaps. Although
        // it might be acceptable because we are writing the progress, we want to play on the safer
        // side unless this really becomes the bottleneck on production.
        {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_transactions___commit"]);
            for batch in batches {
                self.db().write_schemas(batch)?
            }
            Ok(())
        }
    }

    /// Saves signed transaction at `version`.
    pub(crate) fn put_transaction(
        // TODO(grao): Consider remove &self.
        &self,
        version: Version,
        transaction: &Transaction,
        skip_index: bool,
        batch: &mut impl WriteBatch,
    ) -> Result<()> {
        if !skip_index {
            if let Some(txn) = transaction.try_as_signed_user_txn() {
                if let ReplayProtector::SequenceNumber(seq_num) = txn.replay_protector() {
                    batch.put::<OrderedTransactionByAccountSchema>(
                        &(txn.sender(), seq_num),
                        &version,
                    )?;
                }
            }
        }

        let transaction_hash = transaction.hash();

        if let Some(signed_txn) = transaction.try_as_signed_user_txn() {
            let txn_summary = IndexedTransactionSummary::V1 {
                sender: signed_txn.sender(),
                replay_protector: signed_txn.replay_protector(),
                version,
                transaction_hash,
            };
            batch.put::<TransactionSummariesByAccountSchema>(
                &(signed_txn.sender(), version),
                &txn_summary,
            )?;
        }
        batch.put::<TransactionByHashSchema>(&transaction_hash, &version)?;
        batch.put::<TransactionSchema>(&version, transaction)?;

        Ok(())
    }

    /// Deletes transaction data given version range [begin, end).
    pub(crate) fn prune_transactions(
        &self,
        begin: Version,
        end: Version,
        db_batch: &mut SchemaBatch,
    ) -> Result<()> {
        for version in begin..end {
            db_batch.delete::<TransactionSchema>(&version)?;
        }
        Ok(())
    }

    /// Deletes TransactionByHash indices given a list of transactions.
    pub(crate) fn prune_transaction_by_hash_indices(
        &self,
        transaction_hashes: impl Iterator<Item = HashValue>,
        db_batch: &mut SchemaBatch,
    ) -> Result<()> {
        for hash in transaction_hashes {
            db_batch.delete::<TransactionByHashSchema>(&hash)?;
        }
        Ok(())
    }
}
