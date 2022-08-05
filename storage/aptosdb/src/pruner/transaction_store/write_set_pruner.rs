// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::{pruner::db_sub_pruner::DBSubPruner, TransactionStore};
use schemadb::SchemaBatch;
use std::sync::Arc;

#[derive(Debug)]
pub struct WriteSetPruner {
    transaction_store: Arc<TransactionStore>,
}

impl DBSubPruner for WriteSetPruner {
    fn prune(
        &self,
        db_batch: &mut SchemaBatch,
        min_readable_version: u64,
        target_version: u64,
    ) -> anyhow::Result<()> {
        self.transaction_store
            .prune_write_set(min_readable_version, target_version, db_batch)?;
        Ok(())
    }
}

impl WriteSetPruner {
    pub(in crate::pruner) fn new(transaction_store: Arc<TransactionStore>) -> Self {
        WriteSetPruner { transaction_store }
    }
}
