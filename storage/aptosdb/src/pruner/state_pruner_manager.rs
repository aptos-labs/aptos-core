// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module provides `Pruner` which manages a thread pruning old data in the background and is
//! meant to be triggered by other threads as they commit new data to the DB.

use crate::metrics::{PRUNER_BATCH_SIZE, PRUNER_WINDOW};

use aptos_config::config::StoragePrunerConfig;
use aptos_infallible::Mutex;

use crate::pruner::pruner_manager::PrunerManager;
use aptos_types::transaction::Version;
use schemadb::DB;
use std::{
    sync::{
        mpsc::{channel, Sender},
        Arc,
    },
    thread::JoinHandle,
};

use crate::pruner::db_pruner;
use crate::pruner::db_pruner::DBPruner;
use crate::pruner::state_pruner_worker::StatePrunerWorker;
use crate::pruner::state_store::StateStorePruner;
use crate::utils;

/// The `Pruner` is meant to be part of a `AptosDB` instance and runs in the background to prune old
/// data.
///
/// If the state pruner is enabled, it creates a worker thread on construction and joins it on
/// destruction. When destructed, it quits the worker thread eagerly without waiting for all
/// pending work to be done.
#[derive(Debug)]
pub struct StatePrunerManager {
    /// DB version window, which dictates how many versions of state store
    /// to keep.
    prune_window: Option<Version>,
    /// State pruner. Is always initialized regardless if the pruner is enabled to keep tracks
    /// of the min_readable_version.
    pruner: Arc<StateStorePruner>,
    /// The worker thread handle for state_pruner, created upon Pruner instance construction and
    /// joined upon its destruction. It is `None` when state pruner is not enabled or it only
    /// becomes `None` after joined in `drop()`.
    worker_thread: Option<JoinHandle<()>>,
    /// The sender side of the channel talking to the state pruner worker thread. Is `None` when the
    /// state pruner is not enabled.
    command_sender: Option<Mutex<Sender<db_pruner::Command>>>,
    /// We send a batch of version to the underlying pruners for performance reason. This tracks the
    /// last version we sent to the pruner. Will only be set if the pruner is enabled.
    last_version_sent_to_pruner: Arc<Mutex<Version>>,
    /// latest version
    latest_version: Arc<Mutex<Version>>,
}

impl PrunerManager for StatePrunerManager {
    fn get_pruner_window(&self) -> Option<Version> {
        self.prune_window
    }

    fn get_min_readable_version(&self) -> Version {
        self.pruner.as_ref().min_readable_version()
    }

    /// Sends pruning command to the worker thread when necessary.
    fn maybe_wake_pruner(&self, latest_version: Version) {
        *self.latest_version.lock() = latest_version;

        // Always wake up the state pruner.
        if self.prune_window.is_some() {
            self.wake_pruner(latest_version);
            *self.last_version_sent_to_pruner.as_ref().lock() = latest_version;
        }
    }

    fn wake_pruner(&self, latest_version: Version) {
        assert!(self.prune_window.is_some());
        assert!(self.command_sender.is_some());
        self.command_sender
            .as_ref()
            .unwrap()
            .lock()
            .send(db_pruner::Command::Prune {
                target_db_version: latest_version.saturating_sub(self.prune_window.unwrap()),
            })
            .expect("Receiver should not destruct prematurely.");
    }

    /// (For tests only.) Notifies the worker thread and waits for it to finish its job by polling
    /// an internal counter.
    #[cfg(test)]
    fn wake_and_wait_pruner(&self, latest_version: Version) -> anyhow::Result<()> {
        use std::{
            thread::sleep,
            time::{Duration, Instant},
        };

        *self.latest_version.lock() = latest_version;
        self.wake_pruner(latest_version);

        if self.prune_window.is_some() && latest_version > self.prune_window.unwrap() {
            let min_readable_state_store_version = latest_version - self.prune_window.unwrap_or(0);

            // Assuming no big pruning chunks will be issued by a test.
            const TIMEOUT: Duration = Duration::from_secs(10);
            let end = Instant::now() + TIMEOUT;

            while Instant::now() < end {
                if self.get_min_readable_version() >= min_readable_state_store_version {
                    return Ok(());
                }
                sleep(Duration::from_millis(1));
            }
            anyhow::bail!("Timeout waiting for pruner worker.");
        }
        Ok(())
    }
}

impl StatePrunerManager {
    /// Creates a worker thread that waits on a channel for pruning commands.
    pub fn new(state_merkle_rocksdb: Arc<DB>, storage_pruner_config: StoragePrunerConfig) -> Self {
        let state_db_clone = Arc::clone(&state_merkle_rocksdb);
        let state_pruner = utils::create_state_pruner(state_db_clone);

        PRUNER_WINDOW
            .with_label_values(&["state_pruner"])
            .set((storage_pruner_config.state_store_prune_window.unwrap_or(0)) as i64);

        PRUNER_BATCH_SIZE
            .with_label_values(&["state_store_pruner"])
            .set(storage_pruner_config.state_store_pruning_batch_size as i64);

        let mut command_sender = None;

        let state_pruner_worker_thread = if storage_pruner_config.state_store_prune_window.is_some()
        {
            let (state_pruner_command_sender, state_pruner_command_receiver) = channel();
            command_sender = Some(Mutex::new(state_pruner_command_sender));
            let state_pruner_worker = StatePrunerWorker::new(
                Arc::clone(&state_pruner),
                state_pruner_command_receiver,
                storage_pruner_config,
            );
            Some(
                std::thread::Builder::new()
                    .name("aptosdb_state_pruner".into())
                    .spawn(move || state_pruner_worker.work())
                    .expect("Creating state pruner thread should succeed."),
            )
        } else {
            None
        };

        let min_readable_version = state_pruner.as_ref().min_readable_version();
        Self {
            prune_window: storage_pruner_config.state_store_prune_window,
            pruner: state_pruner,
            worker_thread: state_pruner_worker_thread,
            command_sender,
            last_version_sent_to_pruner: Arc::new(Mutex::new(min_readable_version)),
            latest_version: Arc::new(Mutex::new(min_readable_version)),
        }
    }

    #[cfg(test)]
    pub fn testonly_update_min_version(&self, version: Version) {
        self.pruner.testonly_update_min_version(version);
    }
}

impl Drop for StatePrunerManager {
    fn drop(&mut self) {
        if let Some(command_sender) = &self.command_sender {
            command_sender
                .lock()
                .send(db_pruner::Command::Quit)
                .expect("State pruner receiver should not destruct.");
        }
        if self.worker_thread.is_some() {
            self.worker_thread
                .take()
                .expect("Ledger pruner worker thread must exist.")
                .join()
                .expect("Ledger pruner worker thread should join peacefully.");
        }
    }
}
