// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module provides `Pruner` which manages a thread pruning old data in the background and is
//! meant to be triggered by other threads as they commit new data to the DB.

mod db_pruner;
pub(crate) mod db_sub_pruner;
pub(crate) mod event_store;
pub(crate) mod ledger_pruner_worker;
mod ledger_store;
pub(crate) mod state_pruner_worker;
pub(crate) mod state_store;
pub(crate) mod transaction_store;
pub mod utils;

use crate::metrics::{PRUNER_BATCH_SIZE, PRUNER_WINDOW};

use aptos_config::config::StoragePrunerConfig;
use aptos_infallible::Mutex;

use crate::pruner::PrunerIndex::LedgerPrunerIndex;
use aptos_types::transaction::Version;
use ledger_pruner_worker::LedgerPrunerWorker;
use schemadb::DB;
use state_pruner_worker::StatePrunerWorker;
use std::{
    sync::{
        mpsc::{channel, Sender},
        Arc,
    },
    thread::JoinHandle,
};

/// The `Pruner` is meant to be part of a `AptosDB` instance and runs in the background to prune old
/// data.
///
/// It creates a worker thread on construction and joins it on destruction. When destructed, it
/// quits the worker thread eagerly without waiting for all pending work to be done.
#[derive(Debug)]
pub(crate) struct Pruner {
    /// DB version window, which dictates how many versions of state store
    /// to keep.
    state_store_prune_window: Option<Version>,
    /// DB version window, which dictates how many version of other stores like transaction, ledger
    /// info, events etc to keep.
    ledger_prune_window: Option<Version>,
    /// The worker thread handle for state_pruner, created upon Pruner instance construction and
    /// joined upon its destruction. It only becomes `None` after joined in `drop()`.
    state_pruner_worker_thread: Option<JoinHandle<()>>,
    /// The sender side of the channel talking to the state pruner worker thread.
    state_pruner_command_sender: Mutex<Sender<db_pruner::Command>>,
    /// The worker thread handle for ledger_pruner, created upon Pruner instance construction and
    /// joined upon its destruction. It only becomes `None` after joined in `drop()`.
    ledger_pruner_worker_thread: Option<JoinHandle<()>>,
    /// The sender side of the channel talking to the ledger pruner worker thread.
    ledger_pruner_command_sender: Mutex<Sender<db_pruner::Command>>,
    /// A way for the worker thread to inform the `Pruner` the pruning progress. If it
    /// sets value to `V`, all versions before `V` can no longer be accessed. This is protected by
    /// Mutex as this is accessed both by the Pruner thread and the worker thread.
    #[allow(dead_code)]
    state_pruner_min_readable_version: Arc<Mutex<Option<Version>>>,
    ledger_pruner_min_readable_version: Arc<Mutex<Option<Version>>>,
    /// We send a batch of version to the underlying pruners for performance reason. This tracks the
    /// last version we sent to the pruners.
    last_version_sent_to_state_pruner: Arc<Mutex<Version>>,
    last_version_sent_to_ledger_pruner: Arc<Mutex<Version>>,
    /// Ideal batch size of the versions to be sent to the ledger pruner
    ledger_pruner_pruning_batch_size: usize,
    /// latest version
    latest_version: Arc<Mutex<Version>>,
}

pub enum PrunerIndex {
    #[allow(dead_code)]
    StateStorePrunerIndex,
    LedgerPrunerIndex,
}

impl Pruner {
    /// Creates a worker thread that waits on a channel for pruning commands.
    pub fn new(
        ledger_rocksdb: Arc<DB>,
        state_merkle_rocksdb: Arc<DB>,
        storage_pruner_config: StoragePrunerConfig,
    ) -> Self {
        let (state_pruner_command_sender, state_pruner_command_receiver) = channel();
        let (ledger_pruner_command_sender, ledger_pruner_command_receiver) = channel();

        let state_pruner_min_readable_version = Arc::new(Mutex::new(
            storage_pruner_config.state_store_prune_window.map(|_| 0),
        ));

        let state_pruner_min_readable_version_clone =
            Arc::clone(&state_pruner_min_readable_version);

        let ledger_pruner_min_readable_version = Arc::new(Mutex::new(
            storage_pruner_config.ledger_prune_window.map(|_| 0),
        ));

        let ledger_pruner_min_readable_version_clone =
            Arc::clone(&ledger_pruner_min_readable_version);

        PRUNER_WINDOW
            .with_label_values(&["state_pruner"])
            .set((storage_pruner_config.state_store_prune_window.unwrap_or(0)) as i64);

        PRUNER_WINDOW
            .with_label_values(&["ledger_pruner"])
            .set((storage_pruner_config.ledger_prune_window.unwrap_or(0)) as i64);

        PRUNER_BATCH_SIZE
            .with_label_values(&["ledger_pruner"])
            .set(storage_pruner_config.ledger_pruning_batch_size as i64);

        PRUNER_BATCH_SIZE
            .with_label_values(&["state_store_pruner"])
            .set(storage_pruner_config.state_store_pruning_batch_size as i64);

        let state_pruner_worker = StatePrunerWorker::new(
            state_merkle_rocksdb,
            state_pruner_command_receiver,
            state_pruner_min_readable_version,
            storage_pruner_config,
        );

        let ledger_pruner_worker = LedgerPrunerWorker::new(
            ledger_rocksdb,
            ledger_pruner_command_receiver,
            ledger_pruner_min_readable_version,
            storage_pruner_config,
        );
        let state_pruner_worker_thread = std::thread::Builder::new()
            .name("aptosdb_state_pruner".into())
            .spawn(move || state_pruner_worker.work())
            .expect("Creating state pruner thread should succeed.");

        let ledger_pruner_worker_thread = std::thread::Builder::new()
            .name("aptosdb_ledger_pruner".into())
            .spawn(move || ledger_pruner_worker.work())
            .expect("Creating ledger pruner thread should succeed.");

        Self {
            state_store_prune_window: storage_pruner_config.state_store_prune_window,
            ledger_prune_window: storage_pruner_config.ledger_prune_window,
            state_pruner_worker_thread: Some(state_pruner_worker_thread),
            state_pruner_command_sender: Mutex::new(state_pruner_command_sender),
            ledger_pruner_worker_thread: Some(ledger_pruner_worker_thread),
            ledger_pruner_command_sender: Mutex::new(ledger_pruner_command_sender),
            state_pruner_min_readable_version: state_pruner_min_readable_version_clone,
            ledger_pruner_min_readable_version: ledger_pruner_min_readable_version_clone,
            last_version_sent_to_state_pruner: Arc::new(Mutex::new(0)),
            last_version_sent_to_ledger_pruner: Arc::new(Mutex::new(0)),
            ledger_pruner_pruning_batch_size: storage_pruner_config.ledger_pruning_batch_size,
            latest_version: Arc::new(Mutex::new(0)),
        }
    }

    pub fn get_state_store_pruner_window(&self) -> Option<Version> {
        self.state_store_prune_window
    }

    pub fn get_ledger_pruner_window(&self) -> Option<Version> {
        self.ledger_prune_window
    }

    pub fn get_min_readable_version_by_pruner_index(
        &self,
        pruner_index: PrunerIndex,
    ) -> Option<Version> {
        return match pruner_index {
            PrunerIndex::StateStorePrunerIndex => {
                self.state_pruner_min_readable_version.lock().map(|x| x)
            }
            PrunerIndex::LedgerPrunerIndex => {
                self.ledger_pruner_min_readable_version.lock().map(|x| x)
            }
        };
    }

    pub fn get_min_readable_ledger_version(&self) -> Option<Version> {
        self.get_min_readable_version_by_pruner_index(LedgerPrunerIndex)
    }
    /// Sends pruning command to the worker thread when necessary.
    pub fn maybe_wake_pruner(&self, latest_version: Version) {
        *self.latest_version.lock() = latest_version;

        // Always wake up the state pruner.
        self.wake_state_pruner(latest_version);
        *self.last_version_sent_to_state_pruner.as_ref().lock() = latest_version;

        // Only wake up the ledger pruner if there are `ledger_pruner_pruning_batch_size` pending
        // versions.
        if latest_version
            >= *self.last_version_sent_to_ledger_pruner.as_ref().lock()
                + self.ledger_pruner_pruning_batch_size as u64
        {
            self.wake_ledger_pruner(latest_version);
            *self.last_version_sent_to_ledger_pruner.as_ref().lock() = latest_version;
        }
    }

    fn wake_state_pruner(&self, latest_version: Version) {
        self.state_pruner_command_sender
            .lock()
            .send(db_pruner::Command::Prune {
                target_db_version: self
                    .state_store_prune_window
                    .map(|x| latest_version.saturating_sub(x)),
            })
            .expect("Receiver should not destruct prematurely.");
    }

    fn wake_ledger_pruner(&self, latest_version: Version) {
        self.ledger_pruner_command_sender
            .lock()
            .send(db_pruner::Command::Prune {
                target_db_version: self
                    .ledger_prune_window
                    .map(|x| latest_version.saturating_sub(x)),
            })
            .expect("Receiver should not destruct prematurely.");
    }

    /// (For tests only.) Notifies the worker thread and waits for it to finish its job by polling
    /// an internal counter.
    #[cfg(test)]
    pub fn wake_and_wait_state_pruner(&self, latest_version: Version) -> anyhow::Result<()> {
        use std::{
            thread::sleep,
            time::{Duration, Instant},
        };

        *self.latest_version.lock() = latest_version;
        self.wake_state_pruner(latest_version);

        if self.state_store_prune_window.is_some()
            && latest_version > self.state_store_prune_window.unwrap()
        {
            let min_readable_state_store_version =
                latest_version - self.state_store_prune_window.unwrap_or(0);

            // Assuming no big pruning chunks will be issued by a test.
            const TIMEOUT: Duration = Duration::from_secs(10);
            let end = Instant::now() + TIMEOUT;

            while Instant::now() < end {
                if self.state_pruner_min_readable_version.lock().unwrap()
                    >= min_readable_state_store_version
                {
                    return Ok(());
                }
                sleep(Duration::from_millis(1));
            }
            anyhow::bail!("Timeout waiting for pruner worker.");
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn wake_and_wait_ledger_pruner(&self, latest_version: Version) -> anyhow::Result<()> {
        use std::{
            thread::sleep,
            time::{Duration, Instant},
        };

        *self.latest_version.lock() = latest_version;

        if latest_version
            >= *self.last_version_sent_to_ledger_pruner.as_ref().lock()
                + self.ledger_pruner_pruning_batch_size as u64
        {
            self.wake_ledger_pruner(latest_version);
            *self.last_version_sent_to_ledger_pruner.as_ref().lock() = latest_version;
        }

        if self.ledger_prune_window.is_some() && latest_version > self.ledger_prune_window.unwrap()
        {
            let min_readable_ledger_version =
                latest_version - self.ledger_prune_window.unwrap_or(0);

            // Assuming no big pruning chunks will be issued by a test.
            const TIMEOUT: Duration = Duration::from_secs(10);
            let end = Instant::now() + TIMEOUT;

            while Instant::now() < end {
                if self.ledger_pruner_min_readable_version.lock().unwrap()
                    >= min_readable_ledger_version
                {
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
    pub fn ensure_disabled(&self, pruner_index: PrunerIndex) -> anyhow::Result<()> {
        return match pruner_index {
            PrunerIndex::StateStorePrunerIndex => {
                assert!(self.state_pruner_min_readable_version.lock().is_none());
                Ok(())
            }
            PrunerIndex::LedgerPrunerIndex => {
                assert!(self.ledger_pruner_min_readable_version.lock().is_none());
                Ok(())
            }
        };
    }

    /// (For tests only.) Updates the minimal readable version kept by pruner.
    #[cfg(test)]
    pub fn testonly_update_min_version(&mut self, version: &[Option<Version>]) {
        self.state_pruner_min_readable_version = Arc::new(Mutex::new(
            version[PrunerIndex::StateStorePrunerIndex as usize],
        ));
        self.ledger_pruner_min_readable_version =
            Arc::new(Mutex::new(version[PrunerIndex::LedgerPrunerIndex as usize]));
    }
}

impl Drop for Pruner {
    fn drop(&mut self) {
        self.state_pruner_command_sender
            .lock()
            .send(db_pruner::Command::Quit)
            .expect("State pruner receiver should not destruct.");
        self.state_pruner_worker_thread
            .take()
            .expect("State pruner worker thread must exist.")
            .join()
            .expect("State pruner worker thread should join peacefully.");

        self.ledger_pruner_command_sender
            .lock()
            .send(db_pruner::Command::Quit)
            .expect("Ledger pruner receiver should not destruct.");
        self.ledger_pruner_worker_thread
            .take()
            .expect("Ledger pruner worker thread must exist.")
            .join()
            .expect("Ledger pruner worker thread should join peacefully.");
    }
}
