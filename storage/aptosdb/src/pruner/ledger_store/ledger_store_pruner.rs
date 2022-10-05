// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db_metadata::DbMetadataSchema,
    metrics::PRUNER_LEAST_READABLE_VERSION,
    pruner::{
        db_pruner::DBPruner,
        db_sub_pruner::DBSubPruner,
        event_store::event_store_pruner::EventStorePruner,
        state_store::state_value_pruner::StateValuePruner,
        transaction_store::{
            transaction_store_pruner::TransactionStorePruner, write_set_pruner::WriteSetPruner,
        },
    },
    pruner_utils,
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataValue},
        transaction::TransactionSchema,
    },
    EventStore, StateStore, TransactionStore,
};
use aptos_logger::warn;
use aptos_types::transaction::{AtomicVersion, Version};
use schemadb::{ReadOptions, SchemaBatch, DB};
use std::sync::{atomic::Ordering, Arc};

pub const LEDGER_PRUNER_NAME: &str = "ledger_pruner";

#[derive(Debug)]
/// Responsible for pruning everything except for the state tree.
pub(crate) struct LedgerPruner {
    db: Arc<DB>,
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    min_readable_version: AtomicVersion,
    transaction_store_pruner: Arc<dyn DBSubPruner + Send + Sync>,
    state_value_pruner: Arc<dyn DBSubPruner + Send + Sync>,
    event_store_pruner: Arc<dyn DBSubPruner + Send + Sync>,
    write_set_pruner: Arc<dyn DBSubPruner + Send + Sync>,
}

impl DBPruner for LedgerPruner {
    fn name(&self) -> &'static str {
        LEDGER_PRUNER_NAME
    }

    fn prune(&self, max_versions: usize) -> anyhow::Result<Version> {
        if !self.is_pruning_pending() {
            return Ok(self.min_readable_version());
        }

        // Collect the schema batch writes
        let mut db_batch = SchemaBatch::new();
        let current_target_version = self.prune_inner(max_versions, &mut db_batch)?;
        db_batch.put::<DbMetadataSchema>(
            &DbMetadataKey::LedgerPrunerProgress,
            &DbMetadataValue::Version(current_target_version),
        )?;
        // Commit all the changes to DB atomically
        self.db.write_schemas(db_batch)?;

        // TODO(zcc): recording progress after writing schemas might provide wrong answers to
        // API calls when they query min_readable_version while the write_schemas are still in
        // progress.
        self.record_progress(current_target_version);
        Ok(current_target_version)
    }

    fn initialize_min_readable_version(&self) -> anyhow::Result<Version> {
        let stored_min_version = self
            .db
            .get::<DbMetadataSchema>(&DbMetadataKey::LedgerPrunerProgress)?
            .map_or(0, |v| v.expect_version());
        let mut iter = self.db.iter::<TransactionSchema>(ReadOptions::default())?;
        iter.seek(&stored_min_version)?;
        let version = match iter.next().transpose()? {
            Some((version, _)) => version,
            None => 0,
        };
        match version.cmp(&stored_min_version) {
            std::cmp::Ordering::Greater => {
                let res = self.db.put::<DbMetadataSchema>(
                    &DbMetadataKey::LedgerPrunerProgress,
                    &DbMetadataValue::Version(version),
                );
                warn!(
                    stored_min_version = stored_min_version,
                    actual_min_version = version,
                    res = ?res,
                    "Try to update stored min readable transaction version to the actual one.",
                );
                Ok(version)
            }
            std::cmp::Ordering::Equal => Ok(version),
            std::cmp::Ordering::Less => {
                panic!("No transaction is found at or after stored ledger pruner progress ({}), db might be corrupted.", stored_min_version)
            }
        }
    }

    fn min_readable_version(&self) -> Version {
        self.min_readable_version.load(Ordering::Relaxed)
    }

    fn set_target_version(&self, target_version: Version) {
        self.target_version.store(target_version, Ordering::Relaxed)
    }

    fn target_version(&self) -> Version {
        self.target_version.load(Ordering::Relaxed)
    }

    fn record_progress(&self, min_readable_version: Version) {
        self.min_readable_version
            .store(min_readable_version, Ordering::Relaxed);
        PRUNER_LEAST_READABLE_VERSION
            .with_label_values(&["ledger_pruner"])
            .set(min_readable_version as i64);
    }

    /// (For tests only.) Updates the minimal readable version kept by pruner.
    fn testonly_update_min_version(&self, version: Version) {
        self.min_readable_version.store(version, Ordering::Relaxed)
    }
}

impl LedgerPruner {
    pub fn new(
        db: Arc<DB>,
        transaction_store: Arc<TransactionStore>,
        event_store: Arc<EventStore>,
        state_store: Arc<StateStore>,
    ) -> Self {
        let pruner = LedgerPruner {
            db,
            target_version: AtomicVersion::new(0),
            min_readable_version: AtomicVersion::new(0),
            transaction_store_pruner: Arc::new(TransactionStorePruner::new(
                transaction_store.clone(),
            )),
            state_value_pruner: Arc::new(StateValuePruner::new(state_store)),
            event_store_pruner: Arc::new(EventStorePruner::new(event_store)),
            write_set_pruner: Arc::new(WriteSetPruner::new(transaction_store)),
        };
        pruner.initialize();
        pruner
    }

    /// Prunes the genesis transaction and saves the db alterations to the given change set
    pub fn prune_genesis(
        ledger_db: Arc<DB>,
        state_store: Arc<StateStore>,
        db_batch: &mut SchemaBatch,
    ) -> anyhow::Result<()> {
        let target_version = 1; // The genesis version is 0. Delete [0,1) (exclusive)
        let max_version = 1; // We should only be pruning a single version

        let ledger_pruner = pruner_utils::create_ledger_pruner(ledger_db, state_store);
        ledger_pruner.set_target_version(target_version);
        ledger_pruner.prune_inner(max_version, db_batch)?;

        Ok(())
    }

    fn prune_inner(
        &self,
        max_versions: usize,
        db_batch: &mut SchemaBatch,
    ) -> anyhow::Result<Version> {
        let min_readable_version = self.min_readable_version();

        // Current target version might be less than the target version to ensure we don't prune
        // more than max_version in one go.
        let current_target_version = self.get_current_batch_target(max_versions as Version);

        self.transaction_store_pruner.prune(
            db_batch,
            min_readable_version,
            current_target_version,
        )?;
        self.write_set_pruner
            .prune(db_batch, min_readable_version, current_target_version)?;
        self.state_value_pruner
            .prune(db_batch, min_readable_version, current_target_version)?;
        self.event_store_pruner
            .prune(db_batch, min_readable_version, current_target_version)?;

        Ok(current_target_version)
    }
}
