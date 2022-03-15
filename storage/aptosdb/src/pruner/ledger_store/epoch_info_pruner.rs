// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::{
    epoch_by_version::EpochByVersionSchema, metrics::DIEM_PRUNER_LEAST_READABLE_VERSION,
    pruner::db_pruner::DBPruner, LedgerStore,
};
use aptos_types::transaction::{AtomicVersion, Version};
use schemadb::{ReadOptions, SchemaBatch, DB};
use std::sync::{atomic::Ordering, Arc};

pub const EPOCH_INFO_PRUNER_NAME: &str = "epoch info pruner";

pub struct EpochInfoPruner {
    db: Arc<DB>,
    /// Keeps track of the target version that the pruner needs to achieve.
    ledger_store: Arc<LedgerStore>,
    target_version: AtomicVersion,
    /// Keeps track of least readable version for epoch by version store
    least_readable_version: AtomicVersion,
}

impl DBPruner for EpochInfoPruner {
    fn name(&self) -> &'static str {
        EPOCH_INFO_PRUNER_NAME
    }

    fn prune(&self, max_versions: u64) -> anyhow::Result<Version> {
        let current_target_version = self.get_currrent_batch_target(max_versions);
        let mut db_batch = SchemaBatch::new();

        self.ledger_store.prune_epoch_by_version(
            self.least_readable_version(),
            current_target_version,
            &mut db_batch,
        )?;
        self.db.write_schemas(db_batch)?;
        self.record_progress(current_target_version);
        Ok(current_target_version)
    }

    fn initialize_least_readable_version(&self) -> anyhow::Result<Version> {
        let mut iter = self
            .db
            .iter::<EpochByVersionSchema>(ReadOptions::default())?;
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
        DIEM_PRUNER_LEAST_READABLE_VERSION
            .with_label_values(&["epoch_store"])
            .set(least_readable_version as i64);
    }
}

impl EpochInfoPruner {
    pub fn new(db: Arc<DB>, ledger_store: Arc<LedgerStore>) -> Self {
        EpochInfoPruner {
            db,
            ledger_store,
            target_version: AtomicVersion::new(0),
            least_readable_version: AtomicVersion::new(0),
        }
    }
}
