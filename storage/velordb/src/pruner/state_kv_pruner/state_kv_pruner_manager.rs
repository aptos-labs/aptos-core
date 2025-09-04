// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::{PRUNER_BATCH_SIZE, PRUNER_VERSIONS, PRUNER_WINDOW},
    pruner::{
        pruner_manager::PrunerManager, pruner_utils, pruner_worker::PrunerWorker,
        state_kv_pruner::StateKvPruner,
    },
    state_kv_db::StateKvDb,
};
use velor_config::config::LedgerPrunerConfig;
use velor_storage_interface::Result;
use velor_types::transaction::{AtomicVersion, Version};
use std::sync::{atomic::Ordering, Arc};

/// The `PrunerManager` for `StateKvPruner`.
pub(crate) struct StateKvPrunerManager {
    state_kv_db: Arc<StateKvDb>,
    /// DB version window, which dictates how many version of state values to keep.
    prune_window: Version,
    /// It is None iff the pruner is not enabled.
    pruner_worker: Option<PrunerWorker>,
    /// Ideal batch size of the versions to be sent to the state kv pruner.
    pruning_batch_size: usize,
    /// The minimal readable version for the ledger data.
    min_readable_version: AtomicVersion,
}

impl PrunerManager for StateKvPrunerManager {
    type Pruner = StateKvPruner;

    fn is_pruner_enabled(&self) -> bool {
        self.pruner_worker.is_some()
    }

    fn get_prune_window(&self) -> Version {
        self.prune_window
    }

    fn get_min_readable_version(&self) -> Version {
        self.min_readable_version.load(Ordering::SeqCst)
    }

    /// Sets pruner target version when necessary.
    fn maybe_set_pruner_target_db_version(&self, latest_version: Version) {
        let min_readable_version = self.get_min_readable_version();
        // Only wake up the state kv pruner if there are `ledger_pruner_pruning_batch_size` pending
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
            .with_label_values(&["state_kv_pruner", "min_readable"])
            .set(min_readable_version as i64);

        self.state_kv_db.write_pruner_progress(min_readable_version)
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

impl StateKvPrunerManager {
    pub fn new(state_kv_db: Arc<StateKvDb>, state_kv_pruner_config: LedgerPrunerConfig) -> Self {
        let pruner_worker = if state_kv_pruner_config.enable {
            Some(Self::init_pruner(
                Arc::clone(&state_kv_db),
                state_kv_pruner_config,
            ))
        } else {
            None
        };

        let min_readable_version =
            pruner_utils::get_state_kv_pruner_progress(&state_kv_db).expect("Must succeed.");

        PRUNER_VERSIONS
            .with_label_values(&["state_kv_pruner", "min_readable"])
            .set(min_readable_version as i64);

        Self {
            state_kv_db,
            prune_window: state_kv_pruner_config.prune_window,
            pruner_worker,
            pruning_batch_size: state_kv_pruner_config.batch_size,
            min_readable_version: AtomicVersion::new(min_readable_version),
        }
    }

    fn init_pruner(
        state_kv_db: Arc<StateKvDb>,
        state_kv_pruner_config: LedgerPrunerConfig,
    ) -> PrunerWorker {
        let pruner =
            Arc::new(StateKvPruner::new(state_kv_db).expect("Failed to create state kv pruner."));

        PRUNER_WINDOW
            .with_label_values(&["state_kv_pruner"])
            .set(state_kv_pruner_config.prune_window as i64);

        PRUNER_BATCH_SIZE
            .with_label_values(&["state_kv_pruner"])
            .set(state_kv_pruner_config.batch_size as i64);

        PrunerWorker::new(pruner, state_kv_pruner_config.batch_size, "state_kv")
    }

    fn set_pruner_target_db_version(&self, latest_version: Version) {
        assert!(self.pruner_worker.is_some());
        let min_readable_version = latest_version.saturating_sub(self.prune_window);
        self.min_readable_version
            .store(min_readable_version, Ordering::SeqCst);

        PRUNER_VERSIONS
            .with_label_values(&["state_kv_pruner", "min_readable"])
            .set(min_readable_version as i64);

        self.pruner_worker
            .as_ref()
            .unwrap()
            .set_target_db_version(min_readable_version);
    }
}
