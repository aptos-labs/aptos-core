// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod test;

use crate::{
    metrics::DIEM_TRANSACTION_PRUNER_LEAST_READABLE_VERSION, pruner::db_pruner::DBPruner,
    transaction::TransactionSchema,
};
use aptos_logger::{error, info};
use aptos_types::transaction::{AtomicVersion, Version};
use schemadb::{ReadOptions, DB};
use std::{
    cmp::min,
    sync::{atomic::Ordering, Arc},
    thread::sleep,
    time::Duration,
};

pub struct TransactionStorePruner {
    db: Arc<DB>,
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    least_readable_version: AtomicVersion,
}

impl DBPruner for TransactionStorePruner {
    fn initialize(&self) {
        loop {
            match self.initialize_least_readable_version() {
                Ok(least_readable_version) => {
                    info!(
                        least_readable_version = least_readable_version,
                        "[transaction pruner] initialized."
                    );
                    self.record_progress(least_readable_version);
                    return;
                }
                Err(e) => {
                    error!(
                        error = ?e,
                        "[transaction pruner] Error on first seek. Retrying in 1 second.",
                    );
                    sleep(Duration::from_secs(1));
                }
            }
        }
    }

    fn prune(&self, max_versions: usize) -> anyhow::Result<Version> {
        let mut iter = self.db.iter::<TransactionSchema>(ReadOptions::default())?;
        let least_readable_version = self.least_readable_version();
        let current_target_version = min(
            least_readable_version + max_versions as u64,
            self.target_version(),
        );
        iter.seek(&least_readable_version)?;
        self.db.range_delete::<TransactionSchema, Version>(
            &self.least_readable_version(),
            &current_target_version,
        )?;
        self.record_progress(current_target_version);
        Ok(current_target_version)
    }

    fn initialize_least_readable_version(&self) -> anyhow::Result<Version> {
        let mut iter = self.db.iter::<TransactionSchema>(ReadOptions::default())?;
        iter.seek_to_first();
        let version = iter
            .next()
            .transpose()?
            .map_or(0, |(version, _)| return version);
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
        DIEM_TRANSACTION_PRUNER_LEAST_READABLE_VERSION.set(least_readable_version as i64);
    }

    fn is_pruning_pending(&self) -> bool {
        self.least_readable_version() >= self.target_version()
    }
}

impl TransactionStorePruner {
    pub fn new(db: Arc<DB>) -> Self {
        TransactionStorePruner {
            db,
            target_version: AtomicVersion::new(0),
            least_readable_version: AtomicVersion::new(0),
        }
    }
}
