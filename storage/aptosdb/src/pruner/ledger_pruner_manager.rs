// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{PRUNER_BATCH_SIZE, PRUNER_WINDOW};

use aptos_config::config::LedgerPrunerConfig;
use aptos_infallible::Mutex;

use crate::pruner::db_pruner::DBPruner;
use crate::pruner::ledger_pruner_worker::LedgerPrunerWorker;
use crate::pruner::ledger_store::ledger_store_pruner::LedgerPruner;
use crate::pruner::pruner_manager::PrunerManager;
use crate::{pruner_utils, StateStore};
use aptos_types::transaction::Version;
use schemadb::DB;
use std::{sync::Arc, thread::JoinHandle};

/// The `PrunerManager` for `LedgerPruner`.
#[derive(Debug)]
pub(crate) struct LedgerPrunerManager {
    pruner_enabled: bool,
    /// DB version window, which dictates how many version of other stores like transaction, ledger
    /// info, events etc to keep.
    prune_window: Version,
    /// Ledger pruner. Is always initialized regardless if the pruner is enabled to keep tracks
    /// of the min_readable_version.
    pruner: Arc<LedgerPruner>,
    /// Wrapper class of the ledger pruner.
    pruner_worker: Arc<LedgerPrunerWorker>,
    /// The worker thread handle for ledger_pruner, created upon Pruner instance construction and
    /// joined upon its destruction. It is `None` when the ledger pruner is not enabled or it only
    /// becomes `None` after joined in `drop()`.
    worker_thread: Option<JoinHandle<()>>,
    /// We send a batch of version to the underlying pruners for performance reason. This tracks the
    /// last version we sent to the pruners. Will only be set if the pruner is enabled.
    pub(crate) last_version_sent_to_pruner: Arc<Mutex<Version>>,
    /// Ideal batch size of the versions to be sent to the ledger pruner
    pruning_batch_size: usize,
    /// latest version
    latest_version: Arc<Mutex<Version>>,
    /// Offset for displaying to users
    user_pruning_window_offset: u64,
}

impl PrunerManager for LedgerPrunerManager {
    type Pruner = LedgerPruner;

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

        // Only wake up the ledger pruner if there are `ledger_pruner_pruning_batch_size` pending
        // versions.
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

impl LedgerPrunerManager {
    /// Creates a worker thread that waits on a channel for pruning commands.
    pub fn new(
        ledger_rocksdb: Arc<DB>,
        state_store: Arc<StateStore>,
        ledger_pruner_config: LedgerPrunerConfig,
    ) -> Self {
        let ledger_pruner = pruner_utils::create_ledger_pruner(ledger_rocksdb, state_store);

        if ledger_pruner_config.enable {
            PRUNER_WINDOW
                .with_label_values(&["ledger_pruner"])
                .set(ledger_pruner_config.prune_window as i64);

            PRUNER_BATCH_SIZE
                .with_label_values(&["ledger_pruner"])
                .set(ledger_pruner_config.batch_size as i64);
        }

        let ledger_pruner_worker = Arc::new(LedgerPrunerWorker::new(
            Arc::clone(&ledger_pruner),
            ledger_pruner_config,
        ));

        let ledger_pruner_worker_clone = Arc::clone(&ledger_pruner_worker);

        let ledger_pruner_worker_thread = if ledger_pruner_config.enable {
            Some(
                std::thread::Builder::new()
                    .name("aptosdb_ledger_pruner".into())
                    .spawn(move || ledger_pruner_worker_clone.as_ref().work())
                    .expect("Creating ledger pruner thread should succeed."),
            )
        } else {
            None
        };

        let min_readable_version = ledger_pruner.min_readable_version();

        Self {
            pruner_enabled: ledger_pruner_config.enable,
            prune_window: ledger_pruner_config.prune_window,
            pruner: ledger_pruner,
            pruner_worker: ledger_pruner_worker,
            worker_thread: ledger_pruner_worker_thread,
            last_version_sent_to_pruner: Arc::new(Mutex::new(min_readable_version)),
            pruning_batch_size: ledger_pruner_config.batch_size,
            latest_version: Arc::new(Mutex::new(min_readable_version)),
            user_pruning_window_offset: ledger_pruner_config.user_pruning_window_offset,
        }
    }

    #[cfg(test)]
    pub fn testonly_update_min_version(&self, version: Version) {
        self.pruner.testonly_update_min_version(version);
    }
}

impl Drop for LedgerPrunerManager {
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
