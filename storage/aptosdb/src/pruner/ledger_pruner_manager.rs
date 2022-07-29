// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{PRUNER_BATCH_SIZE, PRUNER_WINDOW};

use aptos_config::config::StoragePrunerConfig;
use aptos_infallible::Mutex;

use crate::pruner::db_pruner;
use crate::pruner::ledger_pruner_worker::LedgerPrunerWorker;
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

/// The `PrunerManager` for `LedgerPruner`.
#[derive(Debug)]
pub struct LedgerPrunerManager {
    /// DB version window, which dictates how many version of other stores like transaction, ledger
    /// info, events etc to keep.
    prune_window: Option<Version>,
    /// The worker thread handle for ledger_pruner, created upon Pruner instance construction and
    /// joined upon its destruction. It only becomes `None` after joined in `drop()`.
    pruner_worker_thread: Option<JoinHandle<()>>,
    /// The sender side of the channel talking to the ledger pruner worker thread.
    pruner_command_sender: Mutex<Sender<db_pruner::Command>>,
    /// A way for the worker thread to inform the `Pruner` the pruning progress. If it
    /// sets value to `V`, all versions before `V` can no longer be accessed. This is protected by
    /// Mutex as this is accessed both by the Pruner thread and the worker thread.
    #[allow(dead_code)]
    pruner_min_readable_version: Arc<Mutex<Option<Version>>>,
    /// We send a batch of version to the underlying pruners for performance reason. This tracks the
    /// last version we sent to the pruners.
    pub(crate) last_version_sent_to_pruner: Arc<Mutex<Version>>,
    /// Ideal batch size of the versions to be sent to the ledger pruner
    pruning_batch_size: usize,
    /// latest version
    latest_version: Arc<Mutex<Version>>,
}

impl PrunerManager for LedgerPrunerManager {
    fn get_pruner_window(&self) -> Option<Version> {
        self.prune_window
    }

    fn get_min_readable_version(&self) -> Option<Version> {
        self.pruner_min_readable_version.lock().map(|x| x)
    }

    /// Sends pruning command to the worker thread when necessary.
    fn maybe_wake_pruner(&self, latest_version: Version) {
        *self.latest_version.lock() = latest_version;

        // Only wake up the ledger pruner if there are `ledger_pruner_pruning_batch_size` pending
        // versions.
        if latest_version
            >= *self.last_version_sent_to_pruner.as_ref().lock() + self.pruning_batch_size as u64
        {
            self.wake_pruner(latest_version);
            *self.last_version_sent_to_pruner.as_ref().lock() = latest_version;
        }
    }
    fn wake_pruner(&self, latest_version: Version) {
        self.pruner_command_sender
            .lock()
            .send(db_pruner::Command::Prune {
                target_db_version: self.prune_window.map(|x| latest_version.saturating_sub(x)),
            })
            .expect("Receiver should not destruct prematurely.");
    }

    #[cfg(test)]
    fn wake_and_wait_pruner(&self, latest_version: Version) -> anyhow::Result<()> {
        use std::{
            thread::sleep,
            time::{Duration, Instant},
        };

        *self.latest_version.lock() = latest_version;

        if latest_version
            >= *self.last_version_sent_to_pruner.as_ref().lock() + self.pruning_batch_size as u64
        {
            self.wake_pruner(latest_version);
            *self.last_version_sent_to_pruner.as_ref().lock() = latest_version;
        }

        if self.prune_window.is_some() && latest_version > self.prune_window.unwrap() {
            let min_readable_ledger_version = latest_version - self.prune_window.unwrap_or(0);

            // Assuming no big pruning chunks will be issued by a test.
            const TIMEOUT: Duration = Duration::from_secs(10);
            let end = Instant::now() + TIMEOUT;

            while Instant::now() < end {
                if self.pruner_min_readable_version.lock().unwrap() >= min_readable_ledger_version {
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
        assert!(self.pruner_min_readable_version.lock().is_none());
        Ok(())
    }

    /// (For tests only.) Updates the minimal readable version kept by pruner.
    #[cfg(test)]
    fn testonly_update_min_version(&mut self, version: Option<Version>) {
        self.pruner_min_readable_version = Arc::new(Mutex::new(version));
    }
}

impl LedgerPrunerManager {
    /// Creates a worker thread that waits on a channel for pruning commands.
    pub fn new(ledger_rocksdb: Arc<DB>, storage_pruner_config: StoragePrunerConfig) -> Self {
        let (ledger_pruner_command_sender, ledger_pruner_command_receiver) = channel();

        let ledger_pruner_min_readable_version = Arc::new(Mutex::new(
            storage_pruner_config.ledger_prune_window.map(|_| 0),
        ));

        let ledger_pruner_min_readable_version_clone =
            Arc::clone(&ledger_pruner_min_readable_version);

        PRUNER_WINDOW
            .with_label_values(&["ledger_pruner"])
            .set((storage_pruner_config.ledger_prune_window.unwrap_or(0)) as i64);

        PRUNER_BATCH_SIZE
            .with_label_values(&["ledger_pruner"])
            .set(storage_pruner_config.ledger_pruning_batch_size as i64);

        let ledger_pruner_worker = LedgerPrunerWorker::new(
            ledger_rocksdb,
            ledger_pruner_command_receiver,
            ledger_pruner_min_readable_version,
            storage_pruner_config,
        );

        let ledger_pruner_worker_thread = std::thread::Builder::new()
            .name("aptosdb_ledger_pruner".into())
            .spawn(move || ledger_pruner_worker.work())
            .expect("Creating ledger pruner thread should succeed.");

        Self {
            prune_window: storage_pruner_config.ledger_prune_window,
            pruner_worker_thread: Some(ledger_pruner_worker_thread),
            pruner_command_sender: Mutex::new(ledger_pruner_command_sender),
            pruner_min_readable_version: ledger_pruner_min_readable_version_clone,
            last_version_sent_to_pruner: Arc::new(Mutex::new(0)),
            pruning_batch_size: storage_pruner_config.ledger_pruning_batch_size,
            latest_version: Arc::new(Mutex::new(0)),
        }
    }
}

impl Drop for LedgerPrunerManager {
    fn drop(&mut self) {
        self.pruner_command_sender
            .lock()
            .send(db_pruner::Command::Quit)
            .expect("Ledger pruner receiver should not destruct.");
        self.pruner_worker_thread
            .take()
            .expect("Ledger pruner worker thread must exist.")
            .join()
            .expect("Ledger pruner worker thread should join peacefully.");
    }
}
