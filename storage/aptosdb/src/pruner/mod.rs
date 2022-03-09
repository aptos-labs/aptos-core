// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module provides `Pruner` which manages a thread pruning old data in the background and is
//! meant to be triggered by other threads as they commit new data to the DB.

use crate::metrics::DIEM_STORAGE_PRUNE_WINDOW;

use aptos_config::config::StoragePrunerConfig;
use aptos_infallible::Mutex;

use aptos_types::transaction::Version;
use schemadb::DB;
use std::{
    sync::{
        Arc,
        mpsc::{channel, Sender},
    },
    thread::{JoinHandle, sleep},
    time::{Duration, Instant},
};
use worker::{Command, Worker};

/// The `Pruner` is meant to be part of a `AptosDB` instance and runs in the background to prune old
/// data.
///
/// It creates a worker thread on construction and joins it on destruction. When destructed, it
/// quits the worker thread eagerly without waiting for all pending work to be done.
#[derive(Debug)]
pub(crate) struct Pruner {
    /// DB version window, which dictates how many versions of state store
    /// to keep.
    state_store_prune_window: Version,
    /// DB version window, which dictates how many version of other stoes like transaction, ledger
    /// info, events etc to keep.
    default_prune_window: Version,
    /// The worker thread handle, created upon Pruner instance construction and joined upon its
    /// destruction. It only becomes `None` after joined in `drop()`.
    worker_thread: Option<JoinHandle<()>>,
    /// The sender side of the channel talking to the worker thread.
    command_sender: Mutex<Sender<Command>>,
    /// (For tests) A way for the worker thread to inform the `Pruner` the pruning progress. If it
    /// sets value to `V`, all versions before `V` can no longer be accessed. This is protected by Mutex
    /// as this is accessed both by the Pruner thread and the worker thread.
    #[allow(dead_code)]
    least_readable_version: Arc<Mutex<Vec<Version>>>,
}

impl Pruner {
    /// Creates a worker thread that waits on a channel for pruning commands.
    pub fn new(db: Arc<DB>, storage_pruner_config: StoragePrunerConfig) -> Self {
        let (command_sender, command_receiver) = channel();

        let least_readable_version = Arc::new(Mutex::new(vec![0, 0]));
        let worker_progress_clone = Arc::clone(&least_readable_version);

        DIEM_STORAGE_PRUNE_WINDOW
            .set(storage_pruner_config.state_store_prune_window.unwrap() as i64);
        let worker = Worker::new(db, command_receiver, least_readable_version);
        let worker_thread = std::thread::Builder::new()
            .name("aptosdb_pruner".into())
            .spawn(move || worker.work())
            .expect("Creating pruner thread should succeed.");

        Self {
            state_store_prune_window: storage_pruner_config
                .state_store_prune_window
                .expect("State store prune window must be specified"),
            default_prune_window: storage_pruner_config
                .default_prune_window
                .expect("Default prune window must be specified"),
            worker_thread: Some(worker_thread),
            command_sender: Mutex::new(command_sender),
            least_readable_version: worker_progress_clone,
        }
    }

    pub fn get_state_store_pruner_window(&self) -> Version {
        self.state_store_prune_window.clone()
    }

    /// Sends pruning command to the worker thread when necessary.
    pub fn wake(&self, latest_version: Version) {
        if latest_version > self.state_store_prune_window
            || latest_version > self.default_prune_window
        {
            let least_readable_state_store_version = latest_version - self.state_store_prune_window;
            let least_readable_default_store_version = latest_version - self.default_prune_window;
            self.command_sender
                .lock()
                .send(Command::Prune {
                    target_db_versions: vec![
                        least_readable_state_store_version,
                        least_readable_default_store_version,
                    ],
                })
                .expect("Receiver should not destruct prematurely.");
        }
    }

    /// (For tests only.) Notifies the worker thread and waits for it to finish its job by polling
    /// an internal counter.
    #[cfg(test)]
    pub fn wake_and_wait(&self, latest_version: Version) -> anyhow::Result<()> {
        self.wake(latest_version);

        if latest_version > self.state_store_prune_window
            || latest_version > self.default_prune_window
        {
            let least_readable_state_store_version = latest_version - self.state_store_prune_window;
            // Assuming no big pruning chunks will be issued by a test.
            const TIMEOUT: Duration = Duration::from_secs(60);
            let end = Instant::now() + TIMEOUT;

            while Instant::now() < end {
                if *self.least_readable_version.lock().get(0).unwrap()
                    >= least_readable_state_store_version
                {
                    return Ok(());
                }
                sleep(Duration::from_millis(1));
            }
            anyhow::bail!("Timeout waiting for pruner worker.");
        }
        Ok(())
    }
}

impl Drop for Pruner {
    fn drop(&mut self) {
        self.command_sender
            .lock()
            .send(Command::Quit)
            .expect("Receiver should not destruct.");
        self.worker_thread
            .take()
            .expect("Worker thread must exist.")
            .join()
            .expect("Worker thread should join peacefully.");
    }
}

mod db_pruner;
pub(crate) mod worker;
pub(crate) mod state_store;
pub(crate) mod transaction_store;
