// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::pruner::db_pruner::DBPruner;
use crate::pruner::ledger_store::ledger_store_pruner::LedgerPruner;
use aptos_config::config::StoragePrunerConfig;
use aptos_types::transaction::Version;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

/// Maintains the ledger pruner and periodically calls the db_pruner's prune method to prune the DB.
/// This also exposes API to report the progress to the parent thread.
#[derive(Debug)]
pub struct LedgerPrunerWorker {
    /// The worker will sleep for this period of time after pruning each batch.
    pruning_time_interval_in_ms: u64,
    /// Ledger pruner.
    pruner: Arc<LedgerPruner>,
    /// Max items to prune per batch. For the ledger pruner, this means the max versions to prune
    /// and for the state pruner, this means the max stale nodes to prune.
    max_versions_to_prune_per_batch: u64,
    /// Indicates whether the pruning loop should be running. Will only be set to true on pruner
    /// destruction.
    disable_pruner: AtomicBool,
}

impl LedgerPrunerWorker {
    pub(crate) fn new(
        ledger_pruner: Arc<LedgerPruner>,
        storage_pruner_config: StoragePrunerConfig,
    ) -> Self {
        Self {
            pruning_time_interval_in_ms: storage_pruner_config.ledger_pruner_time_interval_in_ms,
            pruner: ledger_pruner,
            max_versions_to_prune_per_batch: storage_pruner_config.ledger_pruning_batch_size as u64,
            disable_pruner: AtomicBool::new(false),
        }
    }

    // Loop that does the real pruning job.
    pub(crate) fn work(&self) {
        while !self.disable_pruner.load(Ordering::Relaxed) {
            let pruner_result = self
                .pruner
                .prune(self.max_versions_to_prune_per_batch as usize);
            if pruner_result.is_err() {
                println!(
                    "Ledger pruner has error: {:?}",
                    pruner_result.err().unwrap()
                )
            }
            sleep(Duration::from_millis(self.pruning_time_interval_in_ms));
        }
    }

    pub fn set_target_db_version_if_needed(&self, target_db_version: Version) {
        assert!(target_db_version >= self.pruner.target_version());
        self.pruner.set_target_version(target_db_version);
    }

    pub fn stop_pruning(&self) {
        self.disable_pruner.store(true, Ordering::Relaxed);
    }
}
