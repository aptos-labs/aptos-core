// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{PRUNER_BATCH_SIZE, PRUNER_WINDOW};

use aptos_config::config::StoragePrunerConfig;
use aptos_infallible::Mutex;

use crate::pruner::db_pruner;
use crate::pruner::db_pruner::DBPruner;
use crate::pruner::ledger_pruner_worker::LedgerPrunerWorker;
use crate::pruner::ledger_store::ledger_store_pruner::LedgerPruner;
use crate::pruner::pruner_manager::PrunerManager;
use crate::utils;
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
    pruner_enabled: bool,
    /// DB version window, which dictates how many version of other stores like transaction, ledger
    /// info, events etc to keep.
    prune_window: Version,
    /// Ledger pruner. Is always initialized regardless if the pruner is enabled to keep tracks
    /// of the min_readable_version.
    pruner: Arc<LedgerPruner>,
    /// The worker thread handle for ledger_pruner, created upon Pruner instance construction and
    /// joined upon its destruction. It is `None` when the ledger pruner is not enabled or it only
    /// becomes `None` after joined in `drop()`.
    worker_thread: Option<JoinHandle<()>>,
    /// The sender side of the channel talking to the ledger pruner worker thread. Is `None` when
    /// the ledger pruner is not enabled.
    command_sender: Option<Mutex<Sender<db_pruner::Command>>>,
    /// We send a batch of version to the underlying pruners for performance reason. This tracks the
    /// last version we sent to the pruners. Will only be set if the pruner is enabled.
    pub(crate) last_version_sent_to_pruner: Arc<Mutex<Version>>,
    /// Ideal batch size of the versions to be sent to the ledger pruner
    pruning_batch_size: usize,
    /// latest version
    latest_version: Arc<Mutex<Version>>,
}

impl PrunerManager for LedgerPrunerManager {
    fn is_pruner_enabled(&self) -> bool {
        self.pruner_enabled
    }

    fn get_pruner_window(&self) -> Version {
        self.prune_window
    }

    fn get_min_readable_version(&self) -> Version {
        self.pruner.as_ref().min_readable_version()
    }

    /// Sends pruning command to the worker thread when necessary.
    fn maybe_wake_pruner(&self, latest_version: Version) {
        *self.latest_version.lock() = latest_version;

        // Only wake up the ledger pruner if there are `ledger_pruner_pruning_batch_size` pending
        // versions.
        if self.pruner_enabled
            && latest_version
                >= *self.last_version_sent_to_pruner.as_ref().lock()
                    + self.pruning_batch_size as u64
        {
            self.wake_pruner(latest_version);
            *self.last_version_sent_to_pruner.as_ref().lock() = latest_version;
        }
    }
    fn wake_pruner(&self, latest_version: Version) {
        assert!(self.pruner_enabled);
        assert!(self.command_sender.is_some());
        self.command_sender
            .as_ref()
            .unwrap()
            .lock()
            .send(db_pruner::Command::Prune {
                target_db_version: latest_version.saturating_sub(self.prune_window),
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

        if self.pruner_enabled && latest_version > self.prune_window {
            let min_readable_ledger_version = latest_version - self.prune_window;

            // Assuming no big pruning chunks will be issued by a test.
            const TIMEOUT: Duration = Duration::from_secs(10);
            let end = Instant::now() + TIMEOUT;

            while Instant::now() < end {
                if self.get_min_readable_version() >= min_readable_ledger_version {
                    return Ok(());
                }
                sleep(Duration::from_millis(1));
            }
            anyhow::bail!("Timeout waiting for pruner worker.");
        }
        Ok(())
    }
}

impl LedgerPrunerManager {
    /// Creates a worker thread that waits on a channel for pruning commands.
    pub fn new(ledger_rocksdb: Arc<DB>, storage_pruner_config: StoragePrunerConfig) -> Self {
        let ledger_db_clone = Arc::clone(&ledger_rocksdb);

        let ledger_pruner = utils::create_ledger_pruner(ledger_db_clone);

        if storage_pruner_config.enable_ledger_pruner {
            PRUNER_WINDOW
                .with_label_values(&["ledger_pruner"])
                .set(storage_pruner_config.ledger_prune_window as i64);

            PRUNER_BATCH_SIZE
                .with_label_values(&["ledger_pruner"])
                .set(storage_pruner_config.ledger_pruning_batch_size as i64);
        }

        let mut command_sender = None;
        let ledger_pruner_worker_thread = if storage_pruner_config.enable_ledger_pruner {
            let (ledger_pruner_command_sender, ledger_pruner_command_receiver) = channel();
            command_sender = Some(Mutex::new(ledger_pruner_command_sender));
            let ledger_pruner_worker = LedgerPrunerWorker::new(
                Arc::clone(&ledger_pruner),
                ledger_pruner_command_receiver,
                storage_pruner_config,
            );
            Some(
                std::thread::Builder::new()
                    .name("aptosdb_ledger_pruner".into())
                    .spawn(move || ledger_pruner_worker.work())
                    .expect("Creating ledger pruner thread should succeed."),
            )
        } else {
            None
        };

        let min_readable_version = ledger_pruner.min_readable_version();

        Self {
            pruner_enabled: storage_pruner_config.enable_state_store_pruner,
            prune_window: storage_pruner_config.ledger_prune_window,
            pruner: ledger_pruner,
            worker_thread: ledger_pruner_worker_thread,
            command_sender,
            last_version_sent_to_pruner: Arc::new(Mutex::new(min_readable_version)),
            pruning_batch_size: storage_pruner_config.ledger_pruning_batch_size,
            latest_version: Arc::new(Mutex::new(min_readable_version)),
        }
    }

    #[cfg(test)]
    pub fn testonly_update_min_version(&self, version: Version) {
        self.pruner.testonly_update_min_version(version);
    }
}

impl Drop for LedgerPrunerManager {
    fn drop(&mut self) {
        if let Some(command_sender) = &self.command_sender {
            command_sender
                .lock()
                .send(db_pruner::Command::Quit)
                .expect("Ledger pruner receiver should not destruct.");
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
