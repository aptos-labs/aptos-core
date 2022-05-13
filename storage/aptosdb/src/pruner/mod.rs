// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module provides `Pruner` which manages a thread pruning old data in the background and is
//! meant to be triggered by other threads as they commit new data to the DB.

mod db_pruner;
pub(crate) mod db_sub_pruner;
pub(crate) mod event_store;
mod ledger_store;
pub(crate) mod state_store;
pub(crate) mod transaction_store;
pub mod utils;
pub(crate) mod worker;

use crate::metrics::{PRUNER_BATCH_SIZE, PRUNER_WINDOW};

use aptos_config::config::StoragePrunerConfig;
use aptos_infallible::Mutex;

use crate::{EventStore, LedgerStore, TransactionStore};
use aptos_types::transaction::Version;
use schemadb::DB;
use std::{
    sync::{
        mpsc::{channel, Sender},
        Arc,
    },
    thread::JoinHandle,
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
    /// DB version window, which dictates how many version of other stores like transaction, ledger
    /// info, events etc to keep.
    ledger_prune_window: Version,
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
    /// We send a batch of version to the underlying pruners for performance reason. This tracks the
    /// last version we sent to the pruner.
    last_version_sent_to_pruners: Arc<Mutex<Version>>,
    /// Ideal batch size of the versions to be sent to the pruner
    pruning_batch_size: usize,
    /// latest version
    latest_version: Arc<Mutex<Version>>,
}

#[cfg(test)]
pub enum PrunerIndex {
    StateStorePrunerIndex,
    LedgerPrunerIndex,
}

impl Pruner {
    /// Creates a worker thread that waits on a channel for pruning commands.
    pub fn new(
        db: Arc<DB>,
        storage_pruner_config: StoragePrunerConfig,
        transaction_store: Arc<TransactionStore>,
        ledger_store: Arc<LedgerStore>,
        event_store: Arc<EventStore>,
    ) -> Self {
        let (command_sender, command_receiver) = channel();

        let least_readable_version = Arc::new(Mutex::new(vec![0, 0, 0, 0, 0]));
        let worker_progress_clone = Arc::clone(&least_readable_version);

        PRUNER_WINDOW
            .with_label_values(&["state_pruner"])
            .set((storage_pruner_config.state_store_prune_window.unwrap_or(0)) as i64);

        PRUNER_WINDOW
            .with_label_values(&["ledger_pruner"])
            .set((storage_pruner_config.ledger_prune_window.unwrap_or(0)) as i64);

        PRUNER_BATCH_SIZE.set(storage_pruner_config.pruning_batch_size as i64);

        let worker = Worker::new(
            db,
            transaction_store,
            ledger_store,
            event_store,
            command_receiver,
            least_readable_version,
            storage_pruner_config.pruning_batch_size as u64,
        );
        let worker_thread = std::thread::Builder::new()
            .name("aptosdb_pruner".into())
            .spawn(move || worker.work())
            .expect("Creating pruner thread should succeed.");

        Self {
            state_store_prune_window: storage_pruner_config
                .state_store_prune_window
                .expect("State store prune window must be specified"),
            ledger_prune_window: storage_pruner_config
                .ledger_prune_window
                .expect("Default prune window must be specified"),
            worker_thread: Some(worker_thread),
            command_sender: Mutex::new(command_sender),
            least_readable_version: worker_progress_clone,
            last_version_sent_to_pruners: Arc::new(Mutex::new(0)),
            pruning_batch_size: storage_pruner_config.pruning_batch_size,
            latest_version: Arc::new(Mutex::new(0)),
        }
    }

    pub fn get_state_store_pruner_window(&self) -> Version {
        self.state_store_prune_window
    }

    pub fn get_ledger_pruner_window(&self) -> Version {
        self.ledger_prune_window
    }

    /// Sends pruning command to the worker thread when necessary.
    pub fn maybe_wake_pruner(&self, latest_version: Version) {
        *self.latest_version.lock() = latest_version;
        if latest_version
            >= *self.last_version_sent_to_pruners.lock() + self.pruning_batch_size as u64
        {
            self.wake_pruner(latest_version);
            *self.last_version_sent_to_pruners.lock() = latest_version;
        }
    }

    fn wake_pruner(&self, latest_version: Version) {
        let least_readable_state_store_version =
            latest_version.saturating_sub(self.state_store_prune_window);
        let least_readable_ledger_version = latest_version.saturating_sub(self.ledger_prune_window);

        self.command_sender
            .lock()
            .send(Command::Prune {
                target_db_versions: vec![
                    least_readable_state_store_version,
                    least_readable_ledger_version,
                ],
            })
            .expect("Receiver should not destruct prematurely.");
    }

    /// (For tests only.) Notifies the worker thread and waits for it to finish its job by polling
    /// an internal counter.
    #[cfg(test)]
    pub fn wake_and_wait(
        &self,
        latest_version: Version,
        pruner_index: usize,
    ) -> anyhow::Result<()> {
        use std::{
            thread::sleep,
            time::{Duration, Instant},
        };

        self.maybe_wake_pruner(latest_version);

        if latest_version > self.state_store_prune_window
            || latest_version > self.ledger_prune_window
        {
            let least_readable_state_store_version = latest_version - self.state_store_prune_window;
            // Assuming no big pruning chunks will be issued by a test.
            const TIMEOUT: Duration = Duration::from_secs(10);
            let end = Instant::now() + TIMEOUT;

            while Instant::now() < end {
                if *self
                    .least_readable_version
                    .lock()
                    .get(pruner_index)
                    .unwrap()
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
