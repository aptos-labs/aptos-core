// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::{
    metrics::APTOS_PRUNER_LEAST_READABLE_VERSION, pruner::db_pruner::DBPruner,
    write_set::WriteSetSchema, TransactionStore,
};
use aptos_types::transaction::{AtomicVersion, Version};
use schemadb::{ReadOptions, SchemaBatch, DB};
use std::sync::{atomic::Ordering, Arc};

pub const TRANSACTION_STORE_PRUNER_NAME: &str = "transaction store pruner";

pub struct WriteSetPruner {
    db: Arc<DB>,
    transaction_store: Arc<TransactionStore>,
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    least_readable_version: AtomicVersion,
}

impl DBPruner for WriteSetPruner {
    fn name(&self) -> &'static str {
        TRANSACTION_STORE_PRUNER_NAME
    }

    fn prune(&self, db_batch: &mut SchemaBatch, max_versions: u64) -> anyhow::Result<Version> {
        // Current target version  might be less than the target version to ensure we don't prune
        // more than max_version in one go.
        let current_target_version = self.get_currrent_batch_target(max_versions);
        self.transaction_store.prune_write_set(
            self.least_readable_version(),
            current_target_version,
            db_batch,
        )?;

        self.record_progress(current_target_version);
        Ok(current_target_version)
    }

    fn initialize_least_readable_version(&self) -> anyhow::Result<Version> {
        let mut iter = self.db.iter::<WriteSetSchema>(ReadOptions::default())?;
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
        APTOS_PRUNER_LEAST_READABLE_VERSION
            .with_label_values(&["write_set"])
            .set(least_readable_version as i64);
    }
}

impl WriteSetPruner {
    pub(in crate::pruner) fn new(db: Arc<DB>, transaction_store: Arc<TransactionStore>) -> Self {
        WriteSetPruner {
            db,
            transaction_store,
            target_version: AtomicVersion::new(0),
            least_readable_version: AtomicVersion::new(0),
        }
    }
}
