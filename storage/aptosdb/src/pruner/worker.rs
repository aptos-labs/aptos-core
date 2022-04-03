// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use aptos_types::transaction::Version;
use schemadb::{SchemaBatch, DB};

use crate::{
    pruner::{db_pruner::DBPruner, utils},
    EventStore, LedgerStore, TransactionStore,
};
use aptos_infallible::Mutex;
use itertools::zip_eq;
use std::sync::{mpsc::Receiver, Arc};

/// Maintains all the DBPruners and periodically calls the db_pruner's prune method to prune the DB.
/// This also exposes API to report the progress to the parent thread.
pub struct Worker {
    db: Arc<DB>,
    command_receiver: Receiver<Command>,
    /// Keeps tracks of all the DB pruners
    db_pruners: Vec<Mutex<Arc<dyn DBPruner + Send + Sync>>>,
    /// Keeps a record of the pruning progress. If this equals to version `V`, we know versions
    /// smaller than `V` are no longer readable.
    /// This being an atomic value is to communicate the info with the Pruner thread (for tests).
    least_readable_versions: Arc<Mutex<Vec<Version>>>,
    /// Indicates if there's NOT any pending work to do currently, to hint
    /// `Self::receive_commands()` to `recv()` blocking-ly.
    blocking_recv: bool,
    max_version_to_prune_per_batch: u64,
}

impl Worker {
    pub(crate) fn new(
        db: Arc<DB>,
        transaction_store: Arc<TransactionStore>,
        ledger_store: Arc<LedgerStore>,
        event_store: Arc<EventStore>,
        command_receiver: Receiver<Command>,
        least_readable_versions: Arc<Mutex<Vec<Version>>>,
        max_version_to_prune_per_batch: u64,
    ) -> Self {
        let db_pruners =
            utils::create_db_pruners(db.clone(), transaction_store, ledger_store, event_store);
        Self {
            db: Arc::clone(&db),
            db_pruners,
            command_receiver,
            least_readable_versions,
            blocking_recv: true,
            max_version_to_prune_per_batch,
        }
    }

    pub(crate) fn work(mut self) {
        for db_pruner in &self.db_pruners {
            db_pruner.lock().initialize();
        }
        while self.receive_commands() {
            // Process a reasonably small batch of work before trying to receive commands again,
            // in case `Command::Quit` is received (that's when we should quit.)
            let mut error_in_pruning = false;
            let mut db_batch = SchemaBatch::new();
            for db_pruner in &self.db_pruners {
                let result = db_pruner
                    .lock()
                    .prune(&mut db_batch, self.max_version_to_prune_per_batch);
                result.map_err(|_| error_in_pruning = true).ok();
            }
            // Commit all the changes to DB atomically
            let result = self.db.write_schemas(db_batch);
            result.map_err(|_| error_in_pruning = true).ok();
            let mut pruning_pending = false;
            for db_pruner in &self.db_pruners {
                // if any of the pruner has pending pruning, then we don't block on receive
                if db_pruner.lock().is_pruning_pending() {
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
        let mut updated_least_readable_versions: Vec<Version> = Vec::new();
        for x in &self.db_pruners {
            updated_least_readable_versions.push(x.lock().least_readable_version())
        }
        *self.least_readable_versions.lock() = updated_least_readable_versions;
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
                Command::Quit => return false,
                Command::Prune { target_db_versions } => {
                    for (new_target_version, pruner) in
                        zip_eq(&target_db_versions, &self.db_pruners)
                    {
                        if *new_target_version > pruner.lock().target_version() {
                            // Switch to non-blocking to allow some work to be done after the
                            // channel has drained.
                            self.blocking_recv = false;
                        }
                        pruner.lock().set_target_version(*new_target_version);
                    }
                }
            }
        }
    }
}

pub enum Command {
    Quit,
    Prune { target_db_versions: Vec<Version> },
}
