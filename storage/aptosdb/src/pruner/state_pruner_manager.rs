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
use crate::pruner::state_pruner_worker::StatePrunerWorker;

/// The `Pruner` is meant to be part of a `AptosDB` instance and runs in the background to prune old
/// data.
///
/// It creates a worker thread on construction and joins it on destruction. When destructed, it
/// quits the worker thread eagerly without waiting for all pending work to be done.
#[derive(Debug)]
pub struct StatePrunerManager {
    /// DB version window, which dictates how many versions of state store
    /// to keep.
    prune_window: Option<Version>,
    /// The worker thread handle for state_pruner, created upon Pruner instance construction and
    /// joined upon its destruction. It only becomes `None` after joined in `drop()`.
    worker_thread: Option<JoinHandle<()>>,
    /// The sender side of the channel talking to the state pruner worker thread.
    command_sender: Mutex<Sender<db_pruner::Command>>,
    /// A way for the worker thread to inform the `Pruner` the pruning progress. If it
    /// sets value to `V`, all versions before `V` can no longer be accessed. This is protected by
    /// Mutex as this is accessed both by the Pruner thread and the worker thread.
    #[allow(dead_code)]
    min_readable_version: Arc<Mutex<Option<Version>>>,
    /// We send a batch of version to the underlying pruners for performance reason. This tracks the
    /// last version we sent to the pruners.
    last_version_sent_to_pruner: Arc<Mutex<Version>>,
    /// latest version
    latest_version: Arc<Mutex<Version>>,
}

impl PrunerManager for StatePrunerManager {
    fn get_pruner_window(&self) -> Option<Version> {
        self.prune_window
    }

    fn get_min_readable_version(&self) -> Option<Version> {
        self.min_readable_version.lock().map(|x| x)
    }

    /// Sends pruning command to the worker thread when necessary.
    fn maybe_wake_pruner(&self, latest_version: Version) {
        *self.latest_version.lock() = latest_version;

        // Always wake up the state pruner.
        self.wake_pruner(latest_version);
        *self.last_version_sent_to_pruner.as_ref().lock() = latest_version;
    }

    fn wake_pruner(&self, latest_version: Version) {
        self.command_sender
            .lock()
            .send(db_pruner::Command::Prune {
                target_db_version: self.prune_window.map(|x| latest_version.saturating_sub(x)),
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
                if self.min_readable_version.lock().unwrap() >= min_readable_state_store_version {
                    return Ok(());
                }
                sleep(Duration::from_millis(1));
            }
            anyhow::bail!("Timeout waiting for pruner worker.");
        }
        Ok(())
    }

    /// (For tests only.) Ensure a pruner is disabled.
    #[cfg(test)]
    fn ensure_disabled(&self) -> anyhow::Result<()> {
        assert!(self.min_readable_version.lock().is_none());
        Ok(())
    }

    /// (For tests only.) Updates the minimal readable version kept by pruner.
    #[cfg(test)]
    fn testonly_update_min_version(&mut self, version: Option<Version>) {
        self.min_readable_version = Arc::new(Mutex::new(version));
    }
}

impl StatePrunerManager {
    /// Creates a worker thread that waits on a channel for pruning commands.
    pub fn new(state_merkle_rocksdb: Arc<DB>, storage_pruner_config: StoragePrunerConfig) -> Self {
        let (state_pruner_command_sender, state_pruner_command_receiver) = channel();

        let state_pruner_min_readable_version = Arc::new(Mutex::new(
            storage_pruner_config.state_store_prune_window.map(|_| 0),
        ));

        let state_pruner_min_readable_version_clone =
            Arc::clone(&state_pruner_min_readable_version);

        PRUNER_WINDOW
            .with_label_values(&["state_pruner"])
            .set((storage_pruner_config.state_store_prune_window.unwrap_or(0)) as i64);

        PRUNER_BATCH_SIZE
            .with_label_values(&["state_store_pruner"])
            .set(storage_pruner_config.state_store_pruning_batch_size as i64);

        let state_pruner_worker = StatePrunerWorker::new(
            state_merkle_rocksdb,
            state_pruner_command_receiver,
            state_pruner_min_readable_version,
            storage_pruner_config,
        );

        let state_pruner_worker_thread = std::thread::Builder::new()
            .name("aptosdb_state_pruner".into())
            .spawn(move || state_pruner_worker.work())
            .expect("Creating state pruner thread should succeed.");

        Self {
            prune_window: storage_pruner_config.state_store_prune_window,
            worker_thread: Some(state_pruner_worker_thread),
            command_sender: Mutex::new(state_pruner_command_sender),
            min_readable_version: state_pruner_min_readable_version_clone,
            last_version_sent_to_pruner: Arc::new(Mutex::new(0)),
            latest_version: Arc::new(Mutex::new(0)),
        }
    }
}

impl Drop for StatePrunerManager {
    fn drop(&mut self) {
        self.command_sender
            .lock()
            .send(db_pruner::Command::Quit)
            .expect("State pruner receiver should not destruct.");
        self.worker_thread
            .take()
            .expect("State pruner worker thread must exist.")
            .join()
            .expect("State pruner worker thread should join peacefully.");
    }
}
