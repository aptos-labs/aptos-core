// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::{pruner::db_sub_pruner::DBSubPruner, LedgerStore};
use schemadb::SchemaBatch;
use std::sync::Arc;

pub struct LedgerCounterPruner {
    /// Keeps track of the target version that the pruner needs to achieve.
    ledger_store: Arc<LedgerStore>,
}

impl DBSubPruner for LedgerCounterPruner {
    fn prune(
        &self,
        db_batch: &mut SchemaBatch,
        least_readable_version: u64,
        target_version: u64,
    ) -> anyhow::Result<()> {
        self.ledger_store.prune_ledger_counters(
            least_readable_version,
            target_version,
            db_batch,
        )?;
        Ok(())
    }
}

impl LedgerCounterPruner {
    pub fn new(ledger_store: Arc<LedgerStore>) -> Self {
        LedgerCounterPruner { ledger_store }
    }
}
