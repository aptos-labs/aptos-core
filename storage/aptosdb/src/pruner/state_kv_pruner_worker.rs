// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::pruner::{db_pruner::DBPruner, state_kv_pruner::StateKvPruner};
use aptos_config::config::StateKvPrunerConfig;
use aptos_logger::{
    error,
    prelude::{sample, SampleRate},
};
use aptos_types::transaction::Version;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::sleep,
    time::Duration,
};

/// Maintains the state kv pruner and periodically calls the db_pruner's prune method to prune the DB.
/// This also exposes API to report the progress to the parent thread.
pub struct StateKvPrunerWorker {
    /// The worker will sleep for this period of time after pruning each batch.
    pruning_time_interval_in_ms: u64,
    /// State kv pruner.
    pruner: Arc<StateKvPruner>,
    /// Max number of versions to prune per batch.
    max_versions_to_prune_per_batch: u64,
    /// Indicates whether the pruning loop should be running. Will only be set to true on pruner
    /// destruction.
    quit_worker: AtomicBool,
}

impl StateKvPrunerWorker {
    pub(crate) fn new(
        state_kv_pruner: Arc<StateKvPruner>,
        state_kv_pruner_config: StateKvPrunerConfig,
    ) -> Self {
        Self {
            pruning_time_interval_in_ms: if cfg!(test) { 100 } else { 1 },
            pruner: state_kv_pruner,
            max_versions_to_prune_per_batch: state_kv_pruner_config.batch_size as u64,
            quit_worker: AtomicBool::new(false),
        }
    }

    // Loop that does the real pruning job.
    pub(crate) fn work(&self) {
        while !self.quit_worker.load(Ordering::Relaxed) {
            let pruner_result = self
                .pruner
                .prune(self.max_versions_to_prune_per_batch as usize);
            if pruner_result.is_err() {
                sample!(
                    SampleRate::Duration(Duration::from_secs(1)),
                    error!(error = ?pruner_result.err().unwrap(),
                        "State kv pruner has error.")
                );
                sleep(Duration::from_millis(self.pruning_time_interval_in_ms));
                return;
            }
            if !self.pruner.is_pruning_pending() {
                sleep(Duration::from_millis(self.pruning_time_interval_in_ms));
            }
        }
    }

    pub fn set_target_db_version(&self, target_db_version: Version) {
        assert!(target_db_version >= self.pruner.target_version());
        self.pruner.set_target_version(target_db_version);
    }

    pub fn stop_pruning(&self) {
        self.quit_worker.store(true, Ordering::Relaxed);
    }
}
