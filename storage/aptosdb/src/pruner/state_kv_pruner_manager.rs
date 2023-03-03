// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::{PRUNER_BATCH_SIZE, PRUNER_WINDOW},
    pruner::{
        db_pruner::DBPruner, pruner_manager::PrunerManager, state_kv_pruner::StateKvPruner,
        state_kv_pruner_worker::StateKvPrunerWorker,
    },
    pruner_utils,
};
use aptos_config::config::StateKvPrunerConfig;
use aptos_infallible::Mutex;
use aptos_schemadb::DB;
use aptos_types::transaction::Version;
use std::{sync::Arc, thread::JoinHandle};

/// The `PrunerManager` for `StateKvPruner`.
pub(crate) struct StateKvPrunerManager {
    pruner_enabled: bool,
    /// DB version window, which dictates how many version of state values to keep.
    prune_window: Version,
    /// State kv pruner. Is always initialized regardless if the pruner is enabled to keep tracks
    /// of the min_readable_version.
    pruner: Arc<StateKvPruner>,
    /// Wrapper class of the state kv pruner.
    pruner_worker: Arc<StateKvPrunerWorker>,
    /// The worker thread handle for state_kv_pruner, created upon Pruner instance construction and
    /// joined upon its destruction. It is `None` when the state kv pruner is not enabled or it only
    /// becomes `None` after joined in `drop()`.
    worker_thread: Option<JoinHandle<()>>,
    /// We send a batch of version to the underlying pruners for performance reason. This tracks the
    /// last version we sent to the pruners. Will only be set if the pruner is enabled.
    pub(crate) last_version_sent_to_pruner: Arc<Mutex<Version>>,
    /// Ideal batch size of the versions to be sent to the state kv pruner.
    pruning_batch_size: usize,
    /// latest version
    latest_version: Arc<Mutex<Version>>,
}

impl PrunerManager for StateKvPrunerManager {
    type Pruner = StateKvPruner;

    fn pruner(&self) -> &Self::Pruner {
        &self.pruner
    }

    fn is_pruner_enabled(&self) -> bool {
        self.pruner_enabled
    }

    fn get_prune_window(&self) -> Version {
        self.prune_window
    }

    fn get_min_readable_version(&self) -> Version {
        self.pruner.as_ref().min_readable_version()
    }

    fn get_min_viable_version(&self) -> Version {
        unimplemented!()
    }

    /// Sets pruner target version when necessary.
    fn maybe_set_pruner_target_db_version(&self, latest_version: Version) {
        *self.latest_version.lock() = latest_version;

        if self.pruner_enabled
            && latest_version
                >= *self.last_version_sent_to_pruner.as_ref().lock()
                    + self.pruning_batch_size as u64
        {
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

impl StateKvPrunerManager {
    /// Creates a worker thread that waits on a channel for pruning commands.
    pub fn new(state_kv_db: Arc<DB>, state_kv_pruner_config: StateKvPrunerConfig) -> Self {
        let state_kv_pruner = pruner_utils::create_state_kv_pruner(state_kv_db);

        if state_kv_pruner_config.enable {
            PRUNER_WINDOW
                .with_label_values(&["state_kv_pruner"])
                .set(state_kv_pruner_config.prune_window as i64);

            PRUNER_BATCH_SIZE
                .with_label_values(&["state_kv_pruner"])
                .set(state_kv_pruner_config.batch_size as i64);
        }

        let state_kv_pruner_worker = Arc::new(StateKvPrunerWorker::new(
            Arc::clone(&state_kv_pruner),
            state_kv_pruner_config,
        ));

        let state_kv_pruner_worker_clone = Arc::clone(&state_kv_pruner_worker);

        let state_kv_pruner_worker_thread = if state_kv_pruner_config.enable {
            Some(
                std::thread::Builder::new()
                    .name("aptosdb_state_kv_pruner".into())
                    .spawn(move || state_kv_pruner_worker_clone.as_ref().work())
                    .expect("Creating state kv pruner thread should succeed."),
            )
        } else {
            None
        };

        let min_readable_version = state_kv_pruner.min_readable_version();

        Self {
            pruner_enabled: state_kv_pruner_config.enable,
            prune_window: state_kv_pruner_config.prune_window,
            pruner: state_kv_pruner,
            pruner_worker: state_kv_pruner_worker,
            worker_thread: state_kv_pruner_worker_thread,
            last_version_sent_to_pruner: Arc::new(Mutex::new(min_readable_version)),
            pruning_batch_size: state_kv_pruner_config.batch_size,
            latest_version: Arc::new(Mutex::new(min_readable_version)),
        }
    }
}

impl Drop for StateKvPrunerManager {
    fn drop(&mut self) {
        if self.pruner_enabled {
            self.pruner_worker.stop_pruning();

            assert!(self.worker_thread.is_some());
            self.worker_thread
                .take()
                .expect("State kv pruner worker thread must exist.")
                .join()
                .expect("State kv pruner worker thread should join peacefully.");
        }
    }
}
