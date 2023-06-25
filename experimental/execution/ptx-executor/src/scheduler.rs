// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! TODO(aldenhu): doc

use crate::{
    common::{TxnIdx, VersionedKey, BASE_VERSION, EXPECTANT_BLOCK_KEYS},
    executor::PtxExecutorClient,
    state_reader::PtxStateReaderClient,
};
use aptos_types::{
    state_store::state_key::StateKey, transaction::analyzed_transaction::AnalyzedTransaction,
};
use rayon::Scope;
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    sync::mpsc::{channel, Sender},
};

#[derive(Clone)]
pub(crate) struct PtxScheduler;

impl PtxScheduler {
    pub fn spawn(
        scope: &Scope<'_>,
        executor: PtxExecutorClient,
        state_reader: PtxStateReaderClient,
    ) -> PtxSchedulerClient {
        let (work_tx, work_rx) = channel();
        let mut worker = Worker::new(executor, state_reader);
        scope.spawn(move |_scope| loop {
            match work_rx.recv().expect("Channel closed.") {
                Command::AddAnalyzedTransaction(txn) => {
                    worker.add_analyzed_transaction(txn);
                },
                Command::FinishBlock => {
                    worker.finish_block();
                    break;
                },
            }
        });

        PtxSchedulerClient { work_tx }
    }
}

pub(crate) struct PtxSchedulerClient {
    work_tx: Sender<Command>,
}

impl PtxSchedulerClient {
    pub fn add_analyzed_transaction(&self, txn: AnalyzedTransaction) {
        self.send_to_worker(Command::AddAnalyzedTransaction(txn))
    }

    pub fn finish_block(&self) {
        self.send_to_worker(Command::FinishBlock)
    }

    fn send_to_worker(&self, command: Command) {
        self.work_tx.send(command).expect("Work thread died.");
    }
}

enum Command {
    AddAnalyzedTransaction(AnalyzedTransaction),
    FinishBlock,
}

struct Worker {
    executor: PtxExecutorClient,
    state_reader: PtxStateReaderClient,
    latest_writes: HashMap<StateKey, TxnIdx>,
    num_txns: usize,
}

impl Worker {
    fn new(executor: PtxExecutorClient, state_reader: PtxStateReaderClient) -> Self {
        Self {
            executor,
            state_reader,
            latest_writes: HashMap::with_capacity(EXPECTANT_BLOCK_KEYS),
            num_txns: 0,
        }
    }

    fn add_analyzed_transaction(&mut self, txn: AnalyzedTransaction) {
        let txn_index = self.num_txns;
        self.num_txns += 1;

        // TODO(ptx): Reorder Non-P-Transactions. (Now we assume all are P-Txns.)
        let (txn, reads, read_writes) = txn.expect_p_txn();
        let mut dependencies = HashSet::new();
        self.process_txn_dependencies(
            txn_index,
            reads,
            false, /* is_write_set */
            &mut dependencies,
        );
        self.process_txn_dependencies(
            txn_index,
            read_writes,
            true, /* is_write_set */
            &mut dependencies,
        );

        self.executor.add_transaction(txn, dependencies);
    }

    fn process_txn_dependencies(
        &mut self,
        txn_index: TxnIdx,
        keys: Vec<StateKey>,
        is_write_set: bool,
        dependencies: &mut HashSet<VersionedKey>,
    ) {
        for key in keys {
            match self.latest_writes.entry(key.clone()) {
                Entry::Occupied(mut entry) => {
                    dependencies.insert((key.clone(), *entry.get()));
                    if is_write_set {
                        *entry.get_mut() = txn_index;
                    }
                },
                Entry::Vacant(entry) => {
                    dependencies.insert((key.clone(), BASE_VERSION));

                    // TODO(ptx): maybe prioritize reads that unblocks execution immediately.
                    self.state_reader.schedule_read(key);

                    if is_write_set {
                        entry.insert(txn_index);
                    } else {
                        entry.insert(BASE_VERSION);
                    }
                }, // end Entry::Vacant
            } // end match
        } // end for
    }

    fn finish_block(&self) {
        self.executor.finish_block();
        self.state_reader.finish_block();
    }
}
