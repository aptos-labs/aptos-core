// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ledger_db::LedgerDb,
    pruner::{db_sub_pruner::DBSubPruner, pruner_utils::get_or_initialize_subpruner_progress},
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        transaction::TransactionSchema,
    },
    transaction_store::TransactionStore,
};
use aptos_crypto::hash::CryptoHash;
use aptos_db_indexer::db_indexer::InternalIndexerDB;
use aptos_db_indexer_schemas::{
    metadata::{MetadataKey as IndexerMetadataKey, MetadataValue as IndexerMetadataValue},
    schema::indexer_metadata::InternalIndexerMetadataSchema,
};
use aptos_logger::info;
use aptos_schemadb::batch::SchemaBatch;
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use aptos_types::transaction::{Transaction, Version};
use std::sync::Arc;

#[derive(Debug)]
pub struct TransactionPruner {
    transaction_store: Arc<TransactionStore>,
    ledger_db: Arc<LedgerDb>,
    internal_indexer_db: Option<InternalIndexerDB>,
}

impl DBSubPruner for TransactionPruner {
    fn name(&self) -> &str {
        "TransactionPruner"
    }

    fn prune(&self, current_progress: Version, target_version: Version) -> Result<()> {
        let mut batch = SchemaBatch::new();
        let candidate_transactions =
            self.get_pruning_candidate_transactions(current_progress, target_version)?;
        self.ledger_db
            .transaction_db()
            .prune_transaction_by_hash_indices(
                &candidate_transactions
                    .iter()
                    .map(|(_, txn)| txn.hash())
                    .collect::<Vec<_>>(),
                &mut batch,
            )?;
        self.ledger_db.transaction_db().prune_transactions(
            current_progress,
            target_version,
            &mut batch,
        )?;
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::TransactionPrunerProgress,
            &DbMetadataValue::Version(target_version),
        )?;
        if let Some(indexer_db) = self.internal_indexer_db.as_ref() {
            if indexer_db.transaction_enabled() {
                let mut index_batch = SchemaBatch::new();
                self.transaction_store
                    .prune_transaction_by_account(&candidate_transactions, &mut index_batch)?;
                index_batch.put::<InternalIndexerMetadataSchema>(
                    &IndexerMetadataKey::TransactionPrunerProgress,
                    &IndexerMetadataValue::Version(target_version),
                )?;
                indexer_db.get_inner_db_ref().write_schemas(index_batch)?;
            } else {
                self.transaction_store
                    .prune_transaction_by_account(&candidate_transactions, &mut batch)?;
            }
        }
        self.ledger_db.transaction_db().write_schemas(batch)
    }
}

impl TransactionPruner {
    pub(in crate::pruner) fn new(
        transaction_store: Arc<TransactionStore>,
        ledger_db: Arc<LedgerDb>,
        metadata_progress: Version,
        internal_indexer_db: Option<InternalIndexerDB>,
    ) -> Result<Self> {
        let progress = get_or_initialize_subpruner_progress(
            ledger_db.transaction_db_raw(),
            &DbMetadataKey::TransactionPrunerProgress,
            metadata_progress,
        )?;

        let myself = TransactionPruner {
            transaction_store,
            ledger_db,
            internal_indexer_db,
        };

        info!(
            progress = progress,
            metadata_progress = metadata_progress,
            "Catching up TransactionPruner."
        );
        myself.prune(progress, metadata_progress)?;

        Ok(myself)
    }

    fn get_pruning_candidate_transactions(
        &self,
        start: Version,
        end: Version,
    ) -> Result<Vec<(Version, Transaction)>> {
        ensure!(end >= start, "{} must be >= {}", end, start);

        let mut iter = self
            .ledger_db
            .transaction_db_raw()
            .iter::<TransactionSchema>()?;
        iter.seek(&start)?;

        // The capacity is capped by the max number of txns we prune in a single batch. It's a
        // relatively small number set in the config, so it won't cause high memory usage here.
        let mut txns = Vec::with_capacity((end - start) as usize);
        for item in iter {
            let (version, txn) = item?;
            if version >= end {
                break;
            }
            txns.push((version, txn));
        }

        Ok(txns)
    }
}
