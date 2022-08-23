// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module provides `Pruner` which manages a thread pruning old data in the background and is
//! meant to be triggered by other threads as they commit new data to the DB.

use crate::metrics::{PRUNER_BATCH_SIZE, PRUNER_WINDOW};

use aptos_config::config::StateMerklePrunerConfig;
use aptos_infallible::Mutex;

use crate::pruner::pruner_manager::PrunerManager;
use aptos_types::transaction::Version;
use schemadb::DB;
use std::{sync::Arc, thread::JoinHandle};

use crate::pruner::db_pruner::DBPruner;
use crate::pruner::state_pruner_worker::StatePrunerWorker;
use crate::pruner::state_store::StateMerklePruner;
use crate::utils;

/// The `Pruner` is meant to be part of a `AptosDB` instance and runs in the background to prune old
/// data.
///
/// If the state pruner is enabled, it creates a worker thread on construction and joins it on
/// destruction. When destructed, it quits the worker thread eagerly without waiting for all
/// pending work to be done.
#[derive(Debug)]
pub struct StatePrunerManager {
    pruner_enabled: bool,
    /// DB version window, which dictates how many versions of state store
    /// to keep.
    prune_window: Version,
    /// State pruner. Is always initialized regardless if the pruner is enabled to keep tracks
    /// of the min_readable_version.
    pruner: Arc<StateMerklePruner>,
    /// Wrapper class of the state pruner.
    pub(crate) pruner_worker: Arc<StatePrunerWorker>,
    /// The worker thread handle for state_pruner, created upon Pruner instance construction and
    /// joined upon its destruction. It is `None` when state pruner is not enabled or it only
    /// becomes `None` after joined in `drop()`.
    worker_thread: Option<JoinHandle<()>>,
    /// We send a batch of version to the underlying pruners for performance reason. This tracks the
    /// last version we sent to the pruner. Will only be set if the pruner is enabled.
    last_version_sent_to_pruner: Arc<Mutex<Version>>,
    /// latest version
    latest_version: Arc<Mutex<Version>>,
    /// Offset for displaying to users
    user_pruning_window_offset: u64,
}

impl PrunerManager for StatePrunerManager {
    type Pruner = StateMerklePruner;

    fn pruner(&self) -> &Self::Pruner {
        &self.pruner
    }

    fn is_pruner_enabled(&self) -> bool {
        self.pruner_enabled
    }

    fn get_pruner_window(&self) -> Version {
        self.prune_window
    }

    fn get_min_readable_version(&self) -> Version {
        self.pruner.as_ref().min_readable_version()
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

        // Always wake up the state pruner.
        if self.pruner_enabled {
            self.set_pruner_target_db_version(latest_version);
            *self.last_version_sent_to_pruner.as_ref().lock() = latest_version;
        }
    }

    fn set_pruner_target_db_version(&self, latest_version: Version) {
        assert!(self.pruner_enabled);
        self.pruner_worker
            .as_ref()
            .set_target_db_version(latest_version.saturating_sub(self.prune_window));
    }
}

impl StatePrunerManager {
    /// Creates a worker thread that waits on a channel for pruning commands.
    pub fn new(
        state_merkle_rocksdb: Arc<DB>,
        state_merkle_pruner_config: StateMerklePrunerConfig,
    ) -> Self {
        let state_db_clone = Arc::clone(&state_merkle_rocksdb);
        let state_pruner = utils::create_state_pruner(state_db_clone);

        if state_merkle_pruner_config.enable {
            PRUNER_WINDOW
                .with_label_values(&["state_pruner"])
                .set(state_merkle_pruner_config.prune_window as i64);

            PRUNER_BATCH_SIZE
                .with_label_values(&["state_store_pruner"])
                .set(state_merkle_pruner_config.batch_size as i64);
        }

        let state_pruner_worker = Arc::new(StatePrunerWorker::new(
            Arc::clone(&state_pruner),
            state_merkle_pruner_config,
        ));
        let state_pruner_worker_clone = Arc::clone(&state_pruner_worker);

        let state_pruner_worker_thread = if state_merkle_pruner_config.enable {
            Some(
                std::thread::Builder::new()
                    .name("aptosdb_state_pruner".into())
                    .spawn(move || state_pruner_worker_clone.as_ref().work())
                    .expect("Creating state pruner thread should succeed."),
            )
        } else {
            None
        };

        let min_readable_version = state_pruner.as_ref().min_readable_version();
        Self {
            pruner_enabled: state_merkle_pruner_config.enable,
            prune_window: state_merkle_pruner_config.prune_window,
            pruner: state_pruner,
            pruner_worker: state_pruner_worker,
            worker_thread: state_pruner_worker_thread,
            last_version_sent_to_pruner: Arc::new(Mutex::new(min_readable_version)),
            latest_version: Arc::new(Mutex::new(min_readable_version)),
            user_pruning_window_offset: state_merkle_pruner_config.user_pruning_window_offset,
        }
    }

    #[cfg(test)]
    pub fn testonly_update_min_version(&self, version: Version) {
        self.pruner.testonly_update_min_version(version);
    }
}

impl Drop for StatePrunerManager {
    fn drop(&mut self) {
        if self.pruner_enabled {
            self.pruner_worker.stop_pruning();
            assert!(self.worker_thread.is_some());
            self.worker_thread
                .take()
                .expect("Ledger pruner worker thread must exist.")
                .join()
                .expect("Ledger pruner worker thread should join peacefully.");
        }
    }
}
