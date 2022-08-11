// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::{pruner::db_sub_pruner::DBSubPruner, TransactionStore};
use aptos_types::transaction::{Transaction, Version};
use schemadb::SchemaBatch;
use std::sync::Arc;

#[derive(Debug)]
pub struct TransactionStorePruner {
    transaction_store: Arc<TransactionStore>,
}

impl DBSubPruner for TransactionStorePruner {
    fn prune(
        &self,
        db_batch: &mut SchemaBatch,
        min_readable_version: u64,
        target_version: u64,
    ) -> anyhow::Result<()> {
        // Current target version  might be less than the target version to ensure we don't prune
        // more than max_version in one go.

        let candidate_transactions =
            self.get_pruning_candidate_transactions(min_readable_version, target_version)?;
        self.transaction_store
            .prune_transaction_by_hash(&candidate_transactions, db_batch)?;
        self.transaction_store
            .prune_transaction_by_account(&candidate_transactions, db_batch)?;
        self.transaction_store.prune_transaction_schema(
            min_readable_version,
            target_version,
            db_batch,
        )?;
        self.transaction_store.prune_transaction_info_schema(
            min_readable_version,
            target_version,
            db_batch,
        )?;
        self.transaction_store.prune_transaction_accumulator(
            min_readable_version,
            target_version,
            db_batch,
        )?;
        Ok(())
    }
}

impl TransactionStorePruner {
    pub(in crate::pruner) fn new(transaction_store: Arc<TransactionStore>) -> Self {
        TransactionStorePruner { transaction_store }
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
