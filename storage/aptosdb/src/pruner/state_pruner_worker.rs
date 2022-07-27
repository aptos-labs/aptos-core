// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use aptos_types::transaction::Version;
use schemadb::DB;

use crate::pruner::{db_pruner, db_pruner::DBPruner, utils};
use aptos_config::config::StoragePrunerConfig;
use aptos_infallible::Mutex;
use std::sync::{mpsc::Receiver, Arc};

/// Maintains the state store pruner and periodically calls the db_pruner's prune method to prune
/// the DB. This also exposes API to report the progress to the parent thread.
pub struct StatePrunerWorker {
    command_receiver: Receiver<db_pruner::Command>,
    /// State store pruner. If a pruner is not enabled, its value will be None.
    state_pruner: Option<Mutex<Arc<dyn DBPruner + Send + Sync>>>,
    /// Keeps a record of the pruning progress. If this equals to version `V`, we know versions
    /// smaller than `V` are no longer readable.
    /// This being an atomic value is to communicate the info with the Pruner thread (for tests).
    /// If the pruner is disabled, its value will be None.
    min_readable_version: Arc<Mutex<Option<Version>>>,
    /// Indicates if there's NOT any pending work to do currently, to hint
    /// `Self::receive_commands()` to `recv()` blocking-ly.
    blocking_recv: bool,
    /// Max items to prune per batch (i.e. the max stale nodes to prune.)
    state_store_max_nodes_to_prune_per_batch: u64,
}

impl StatePrunerWorker {
    pub(crate) fn new(
        state_merkle_db: Arc<DB>,
        command_receiver: Receiver<db_pruner::Command>,
        min_readable_version: Arc<Mutex<Option<Version>>>,
        storage_pruner_config: StoragePrunerConfig,
    ) -> Self {
        let state_pruner = utils::create_state_pruner(state_merkle_db, storage_pruner_config);
        Self {
            state_pruner,
            command_receiver,
            min_readable_version,
            blocking_recv: true,
            state_store_max_nodes_to_prune_per_batch: storage_pruner_config
                .state_store_pruning_batch_size
                as u64,
        }
    }

    pub(crate) fn work(mut self) {
        while self.receive_commands() {
            // Process a reasonably small batch of work before trying to receive commands again,
            // in case `Command::Quit` is received (that's when we should quit.)
            let mut error_in_pruning = false;
            let mut pruning_pending = false;

            if let Some(state_pruner) = &self.state_pruner {
                let state_store_pruner = state_pruner.lock();
                state_store_pruner
                    .prune(self.state_store_max_nodes_to_prune_per_batch as usize)
                    .map_err(|_| error_in_pruning = true)
                    .ok();

                if state_store_pruner.is_pruning_pending() {
                    pruning_pending = true;
                }
            }

            if !pruning_pending || error_in_pruning {
                self.blocking_recv = true;
            } else {
                self.blocking_recv = false;
            }
            self.record_progress();
        }
    }

    fn record_progress(&mut self) {
        *self.min_readable_version.lock() = self
            .state_pruner
            .as_ref()
            .map(|state_pruner| state_pruner.lock().min_readable_version());
    }

    /// Tries to receive all pending commands, blocking waits for the next command if no work needs
    /// to be done, otherwise quits with `true` to allow the outer loop to do some work before
    /// getting back here.
    ///
    /// Returns `false` if `Command::Quit` is received, to break the outer loop and let
    /// `work_loop()` return.
    fn receive_commands(&mut self) -> bool {
        loop {
            let command = if self.blocking_recv {
                // Worker has nothing to do, blocking wait for the next command.
                self.command_receiver
                    .recv()
                    .expect("Sender should not destruct prematurely.")
            } else {
                // Worker has pending work to do, non-blocking recv.
                match self.command_receiver.try_recv() {
                    Ok(command) => command,
                    // Channel has drained, yield control to the outer loop.
                    Err(_) => return true,
                }
            };

            match command {
                // On `Command::Quit` inform the outer loop to quit by returning `false`.
                db_pruner::Command::Quit => return false,
                db_pruner::Command::Prune { target_db_version } => {
                    if let Some(state_pruner_target_version) = target_db_version {
                        if let Some(state_pruner) = &self.state_pruner {
                            if state_pruner_target_version > state_pruner.lock().target_version() {
                                // Switch to non-blocking to allow some work to be done after the
                                // channel has drained.
                                self.blocking_recv = false;
                            }
                            state_pruner
                                .lock()
                                .set_target_version(state_pruner_target_version);
                        }
                    }
                }
            }
        }
    }
}
