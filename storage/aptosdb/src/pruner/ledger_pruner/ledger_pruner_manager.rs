// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ledger_db::LedgerDb,
    metrics::{PRUNER_BATCH_SIZE, PRUNER_VERSIONS, PRUNER_WINDOW},
    pruner::{
        ledger_pruner::LedgerPruner, pruner_manager::PrunerManager, pruner_utils,
        pruner_worker::PrunerWorker,
    },
};
use aptos_config::config::LedgerPrunerConfig;
use aptos_db_indexer::db_indexer::InternalIndexerDB;
use aptos_infallible::Mutex;
use aptos_storage_interface::Result;
use aptos_types::transaction::{AtomicVersion, Version};
use std::sync::{atomic::Ordering, Arc};

/// The `PrunerManager` for `LedgerPruner`.
pub(crate) struct LedgerPrunerManager {
    ledger_db: Arc<LedgerDb>,
    /// DB version window, which dictates how many version of other stores like transaction, ledger
    /// info, events etc to keep.
    prune_window: Version,
    /// It is None iff the pruner is not enabled.
    pruner_worker: Option<PrunerWorker>,
    /// Ideal batch size of the versions to be sent to the ledger pruner
    pruning_batch_size: usize,
    /// latest version
    latest_version: Arc<Mutex<Version>>,
    /// Offset for displaying to users
    user_pruning_window_offset: u64,
    /// The minimal readable version for the ledger data.
    min_readable_version: AtomicVersion,
}

impl PrunerManager for LedgerPrunerManager {
    type Pruner = LedgerPruner;

    fn is_pruner_enabled(&self) -> bool {
        self.pruner_worker.is_some()
    }

    fn get_prune_window(&self) -> Version {
        self.prune_window
    }

    fn get_min_readable_version(&self) -> Version {
        self.min_readable_version.load(Ordering::SeqCst)
    }

    fn get_min_viable_version(&self) -> Version {
        let min_version = self.get_min_readable_version();
        if self.is_pruner_enabled() {
            let adjusted_window = self
                .prune_window
                .saturating_sub(self.user_pruning_window_offset);
            let adjusted_cutoff = self.latest_version.lock().saturating_sub(adjusted_window);
            std::cmp::max(min_version, adjusted_cutoff)
        } else {
            min_version
        }
    }

    /// Sets pruner target version when necessary.
    fn maybe_set_pruner_target_db_version(&self, latest_version: Version) {
        *self.latest_version.lock() = latest_version;

        let min_readable_version = self.get_min_readable_version();
        // Only wake up the ledger pruner if there are `ledger_pruner_pruning_batch_size` pending
        // versions.
        if self.is_pruner_enabled()
            && latest_version
                >= min_readable_version + self.pruning_batch_size as u64 + self.prune_window
        {
            self.set_pruner_target_db_version(latest_version);
        }
    }

    fn save_min_readable_version(&self, min_readable_version: Version) -> Result<()> {
        self.min_readable_version
            .store(min_readable_version, Ordering::SeqCst);

        PRUNER_VERSIONS
            .with_label_values(&["ledger_pruner", "min_readable"])
            .set(min_readable_version as i64);

        self.ledger_db.write_pruner_progress(min_readable_version)
    }

    fn is_pruning_pending(&self) -> bool {
        self.pruner_worker
            .as_ref()
            .is_some_and(|w| w.is_pruning_pending())
    }

    #[cfg(test)]
    fn set_worker_target_version(&self, target_version: Version) {
        self.pruner_worker
            .as_ref()
            .unwrap()
            .set_target_db_version(target_version);
    }
}

impl LedgerPrunerManager {
    /// Creates a worker thread that waits on a channel for pruning commands.
    pub fn new(
        ledger_db: Arc<LedgerDb>,
        ledger_pruner_config: LedgerPrunerConfig,
        internal_indexer_db: Option<InternalIndexerDB>,
    ) -> Self {
        let pruner_worker = if ledger_pruner_config.enable {
            Some(Self::init_pruner(
                Arc::clone(&ledger_db),
                ledger_pruner_config,
                internal_indexer_db,
            ))
        } else {
            None
        };

        let min_readable_version =
            pruner_utils::get_ledger_pruner_progress(&ledger_db).expect("Must succeed.");

        PRUNER_VERSIONS
            .with_label_values(&["ledger_pruner", "min_readable"])
            .set(min_readable_version as i64);

        Self {
            ledger_db,
            prune_window: ledger_pruner_config.prune_window,
            pruner_worker,
            pruning_batch_size: ledger_pruner_config.batch_size,
            latest_version: Arc::new(Mutex::new(min_readable_version)),
            user_pruning_window_offset: ledger_pruner_config.user_pruning_window_offset,
            min_readable_version: AtomicVersion::new(min_readable_version),
        }
    }

    fn init_pruner(
        ledger_db: Arc<LedgerDb>,
        ledger_pruner_config: LedgerPrunerConfig,
        internal_indexer_db: Option<InternalIndexerDB>,
    ) -> PrunerWorker {
        let pruner = Arc::new(
            LedgerPruner::new(ledger_db, internal_indexer_db)
                .expect("Failed to create ledger pruner."),
        );

        PRUNER_WINDOW
            .with_label_values(&["ledger_pruner"])
            .set(ledger_pruner_config.prune_window as i64);

        PRUNER_BATCH_SIZE
            .with_label_values(&["ledger_pruner"])
            .set(ledger_pruner_config.batch_size as i64);

        PrunerWorker::new(pruner, ledger_pruner_config.batch_size, "ledger")
    }

    fn set_pruner_target_db_version(&self, latest_version: Version) {
        assert!(self.pruner_worker.is_some());
        let min_readable_version = latest_version.saturating_sub(self.prune_window);
        self.min_readable_version
            .store(min_readable_version, Ordering::SeqCst);

        PRUNER_VERSIONS
            .with_label_values(&["ledger_pruner", "min_readable"])
            .set(min_readable_version as i64);

        self.pruner_worker
            .as_ref()
            .unwrap()
            .set_target_db_version(min_readable_version);
    }
}
