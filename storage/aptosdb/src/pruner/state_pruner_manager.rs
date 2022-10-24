// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module provides `Pruner` which manages a thread pruning old data in the background and is
//! meant to be triggered by other threads as they commit new data to the DB.

use crate::metrics::{PRUNER_BATCH_SIZE, PRUNER_WINDOW};

use aptos_config::config::StateMerklePrunerConfig;
use aptos_infallible::Mutex;

use crate::pruner::pruner_manager::PrunerManager;
use aptos_jellyfish_merkle::StaleNodeIndex;
use aptos_types::transaction::Version;
use schemadb::schema::KeyCodec;
use schemadb::DB;
use std::{sync::Arc, thread::JoinHandle};

use crate::pruner::db_pruner::DBPruner;
use crate::pruner::state_pruner_worker::StatePrunerWorker;
use crate::pruner::state_store::generics::StaleNodeIndexSchemaTrait;
use crate::pruner::state_store::StateMerklePruner;
use crate::pruner_utils;

/// The `Pruner` is meant to be part of a `AptosDB` instance and runs in the background to prune old
/// data.
///
/// If the state pruner is enabled, it creates a worker thread on construction and joins it on
/// destruction. When destructed, it quits the worker thread eagerly without waiting for all
/// pending work to be done.
#[derive(Debug)]
pub struct StatePrunerManager<S: StaleNodeIndexSchemaTrait>
where
    StaleNodeIndex: KeyCodec<S>,
{
    pruner_enabled: bool,
    /// DB version window, which dictates how many versions of state store
    /// to keep.
    prune_window: Version,
    /// State pruner. Is always initialized regardless if the pruner is enabled to keep tracks
    /// of the min_readable_version.
    pruner: Arc<StateMerklePruner<S>>,
    /// Wrapper class of the state pruner.
    pub(crate) pruner_worker: Arc<StatePrunerWorker<S>>,
    /// The worker thread handle for state_pruner, created upon Pruner instance construction and
    /// joined upon its destruction. It is `None` when state pruner is not enabled or it only
    /// becomes `None` after joined in `drop()`.
    worker_thread: Option<JoinHandle<()>>,
    /// We send a batch of version to the underlying pruners for performance reason. This tracks the
    /// last version we sent to the pruner. Will only be set if the pruner is enabled.
    last_version_sent_to_pruner: Arc<Mutex<Version>>,
    /// latest version
    latest_version: Arc<Mutex<Version>>,
}

impl<S: StaleNodeIndexSchemaTrait> PrunerManager for StatePrunerManager<S>
where
    StaleNodeIndex: KeyCodec<S>,
{
    type Pruner = StateMerklePruner<S>;

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

impl<S: StaleNodeIndexSchemaTrait> StatePrunerManager<S>
where
    StaleNodeIndex: KeyCodec<S>,
{
    /// Creates a worker thread that waits on a channel for pruning commands.
    pub fn new(state_merkle_rocksdb: Arc<DB>, config: StateMerklePrunerConfig) -> Self {
        let state_db_clone = Arc::clone(&state_merkle_rocksdb);
        let pruner = pruner_utils::create_state_pruner(state_db_clone);

        if config.enable {
            PRUNER_WINDOW
                .with_label_values(&[S::name()])
                .set(config.prune_window as i64);

            PRUNER_BATCH_SIZE
                .with_label_values(&[S::name()])
                .set(config.batch_size as i64);
        }

        let pruner_worker = Arc::new(StatePrunerWorker::new(Arc::clone(&pruner), config));
        let state_pruner_worker_clone = Arc::clone(&pruner_worker);

        let worker_thread = if config.enable {
            Some(
                std::thread::Builder::new()
                    .name("aptosdb_state_pruner".into())
                    .spawn(move || state_pruner_worker_clone.as_ref().work())
                    .expect("Creating state pruner thread should succeed."),
            )
        } else {
            None
        };

        let min_readable_version = pruner.as_ref().min_readable_version();
        Self {
            pruner_enabled: config.enable,
            prune_window: config.prune_window,
            pruner,
            pruner_worker,
            worker_thread,
            last_version_sent_to_pruner: Arc::new(Mutex::new(min_readable_version)),
            latest_version: Arc::new(Mutex::new(min_readable_version)),
        }
    }

    #[cfg(test)]
    pub fn testonly_update_min_version(&self, version: Version) {
        self.pruner.testonly_update_min_version(version);
    }
}

impl<S: StaleNodeIndexSchemaTrait> Drop for StatePrunerManager<S>
where
    StaleNodeIndex: KeyCodec<S>,
{
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
