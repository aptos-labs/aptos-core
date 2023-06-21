// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pruner::{
        db_sub_pruner::DBSubPruner, pruner_utils::get_or_initialize_ledger_subpruner_progress,
    },
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
    TransactionStore,
};
use anyhow::Result;
use aptos_schemadb::{SchemaBatch, DB};
use aptos_types::transaction::{Transaction, Version};
use std::sync::Arc;

#[derive(Debug)]
pub struct TransactionPruner {
    transaction_store: Arc<TransactionStore>,
    transaction_db: Arc<DB>,
}

impl DBSubPruner for TransactionPruner {
    fn prune(&self, current_progress: Version, target_version: Version) -> Result<()> {
        let batch = SchemaBatch::new();
        let candidate_transactions =
            self.get_pruning_candidate_transactions(current_progress, target_version)?;
        self.transaction_store
            .prune_transaction_by_hash(&candidate_transactions, &batch)?;
        self.transaction_store
            .prune_transaction_by_account(&candidate_transactions, &batch)?;
        self.transaction_store.prune_transaction_schema(
            current_progress,
            target_version,
            &batch,
        )?;
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::TransactionPrunerProgress,
            &DbMetadataValue::Version(target_version),
        )?;
        self.transaction_db.write_schemas(batch)
    }
}

impl TransactionPruner {
    pub(in crate::pruner) fn new(
        transaction_store: Arc<TransactionStore>,
        transaction_db: Arc<DB>,
        metadata_progress: Version,
    ) -> Result<Self> {
        let progress = get_or_initialize_ledger_subpruner_progress(
            &transaction_db,
            &DbMetadataKey::TransactionPrunerProgress,
            metadata_progress,
        )?;

        let myself = TransactionPruner {
            transaction_store,
            transaction_db,
        };

        myself.prune(progress, metadata_progress)?;

        Ok(myself)
    }

    fn get_pruning_candidate_transactions(
        &self,
        start: Version,
        end: Version,
    ) -> anyhow::Result<Vec<Transaction>> {
        self.transaction_store
            .get_transaction_iter(start, (end - start) as usize)?
            .collect()
    }
}
