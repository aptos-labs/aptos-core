// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::pruner::ledger_store::ledger_store_pruner::LedgerPruner;
use crate::pruner::{db_pruner, db_pruner::DBPruner};
use aptos_config::config::StoragePrunerConfig;
use std::sync::{mpsc::Receiver, Arc};

/// Maintains the ledger pruner and periodically calls the db_pruner's prune method to prune the DB.
/// This also exposes API to report the progress to the parent thread.
pub struct LedgerPrunerWorker {
    command_receiver: Receiver<db_pruner::Command>,
    /// Ledger pruner.
    pruner: Arc<LedgerPruner>,
    /// Indicates if there's NOT any pending work to do currently, to hint
    /// `Self::receive_commands()` to `recv()` blocking-ly.
    blocking_recv: bool,
    /// Max items to prune per batch. For the ledger pruner, this means the max versions to prune
    /// and for the state pruner, this means the max stale nodes to prune.
    ledger_store_max_versions_to_prune_per_batch: u64,
}

impl LedgerPrunerWorker {
    pub(crate) fn new(
        ledger_pruner: Arc<LedgerPruner>,
        command_receiver: Receiver<db_pruner::Command>,
        storage_pruner_config: StoragePrunerConfig,
    ) -> Self {
        Self {
            pruner: ledger_pruner,
            command_receiver,
            blocking_recv: true,
            ledger_store_max_versions_to_prune_per_batch: storage_pruner_config
                .ledger_pruning_batch_size
                as u64,
        }
    }

    pub(crate) fn work(mut self) {
        while self.receive_commands() {
            // Process a reasonably small batch of work before trying to receive commands again,
            // in case `Command::Quit` is received (that's when we should quit.)
            let mut error_in_pruning = false;

            self.pruner
                .prune(self.ledger_store_max_versions_to_prune_per_batch as usize)
                .map_err(|_| error_in_pruning = true)
                .ok();

            if !self.pruner.is_pruning_pending() || error_in_pruning {
                self.blocking_recv = true;
            } else {
                self.blocking_recv = false;
            }
        }
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
                // LedgerPrunerWorker has nothing to do, blocking wait for the next command.
                self.command_receiver
                    .recv()
                    .expect("Sender should not destruct prematurely.")
            } else {
                // LedgerPrunerWorker has pending work to do, non-blocking recv.
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
                    if target_db_version > self.pruner.target_version() {
                        // Switch to non-blocking to allow some work to be done after the
                        // channel has drained.
                        self.blocking_recv = false;
                    }
                    self.pruner.set_target_version(target_db_version);
                }
            }
        }
    }
}
