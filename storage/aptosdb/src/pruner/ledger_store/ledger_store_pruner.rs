// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::{
    metrics::PRUNER_LEAST_READABLE_VERSION,
    pruner::{
        db_pruner::DBPruner,
        db_sub_pruner::DBSubPruner,
        event_store::event_store_pruner::EventStorePruner,
        ledger_store::ledger_counter_pruner::LedgerCounterPruner,
        transaction_store::{
            transaction_store_pruner::TransactionStorePruner, write_set_pruner::WriteSetPruner,
        },
    },
    transaction::TransactionSchema,
    EventStore, LedgerStore, TransactionStore,
};
use aptos_types::transaction::{AtomicVersion, Version};
use schemadb::{ReadOptions, SchemaBatch, DB};
use std::sync::{atomic::Ordering, Arc};

pub const LEDGER_PRUNER_NAME: &str = "ledger pruner";

pub struct LedgerPruner {
    db: Arc<DB>,
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    least_readable_version: AtomicVersion,
    transaction_store_pruner: Arc<dyn DBSubPruner + Send + Sync>,
    event_store_pruner: Arc<dyn DBSubPruner + Send + Sync>,
    write_set_pruner: Arc<dyn DBSubPruner + Send + Sync>,
    ledger_counter_pruner: Arc<dyn DBSubPruner + Send + Sync>,
}

impl DBPruner for LedgerPruner {
    fn name(&self) -> &'static str {
        LEDGER_PRUNER_NAME
    }

    fn prune(&self, db_batch: &mut SchemaBatch, max_versions: u64) -> anyhow::Result<Version> {
        if !self.is_pruning_pending() {
            return Ok(self.least_readable_version());
        }
        let least_readable_version = self.least_readable_version();
        // Current target version might be less than the target version to ensure we don't prune
        // more than max_version in one go.
        let current_target_version = self.get_currrent_batch_target(max_versions);

        self.transaction_store_pruner.prune(
            db_batch,
            least_readable_version,
            current_target_version,
        )?;
        self.write_set_pruner
            .prune(db_batch, least_readable_version, current_target_version)?;
        self.ledger_counter_pruner.prune(
            db_batch,
            least_readable_version,
            current_target_version,
        )?;

        self.event_store_pruner
            .prune(db_batch, least_readable_version, current_target_version)?;

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
        PRUNER_LEAST_READABLE_VERSION
            .with_label_values(&["ledger_pruner"])
            .set(least_readable_version as i64);
    }
}

impl LedgerPruner {
    pub(in crate::pruner) fn new(
        db: Arc<DB>,
        transaction_store: Arc<TransactionStore>,
        event_store: Arc<EventStore>,
        ledger_store: Arc<LedgerStore>,
    ) -> Self {
        LedgerPruner {
            db,
            target_version: AtomicVersion::new(0),
            least_readable_version: AtomicVersion::new(0),
            ledger_counter_pruner: Arc::new(LedgerCounterPruner::new(ledger_store)),
            transaction_store_pruner: Arc::new(TransactionStorePruner::new(
                transaction_store.clone(),
            )),
            event_store_pruner: Arc::new(EventStorePruner::new(event_store)),
            write_set_pruner: Arc::new(WriteSetPruner::new(transaction_store)),
        }
    }
}
