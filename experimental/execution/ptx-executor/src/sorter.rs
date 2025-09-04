// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! TODO(aldenhu): doc

use crate::{
    common::{Entry, HashMap, HashSet, TxnIdx, VersionedKey, BASE_VERSION, EXPECTANT_BLOCK_KEYS},
    metrics::TIMER,
    scheduler::PtxSchedulerClient,
    state_reader::PtxStateReaderClient,
};
use velor_logger::trace;
use velor_metrics_core::TimerHelper;
use velor_types::{
    state_store::state_key::StateKey, transaction::analyzed_transaction::AnalyzedTransaction,
};
use rayon::Scope;
use std::sync::mpsc::{channel, Sender};

#[derive(Clone)]
pub(crate) struct PtxSorter;

impl PtxSorter {
    pub fn spawn(
        scope: &Scope<'_>,
        scheduler: PtxSchedulerClient,
        state_reader: PtxStateReaderClient,
    ) -> PtxSorterClient {
        let (work_tx, work_rx) = channel();
        let mut worker = Worker::new(scheduler, state_reader);
        scope.spawn(move |_scope| {
            let _timer = TIMER.timer_with(&["sorter_block_total"]);
            loop {
                match work_rx.recv().expect("Channel closed.") {
                    Command::AddAnalyzedTransaction(txn) => {
                        worker.add_analyzed_transaction(txn);
                    },
                    Command::FinishBlock => {
                        worker.finish_block();
                        trace!("finish_block.");
                        break;
                    },
                }
            }
        });

        PtxSorterClient { work_tx }
    }
}

pub(crate) struct PtxSorterClient {
    work_tx: Sender<Command>,
}

impl PtxSorterClient {
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
    scheduler: PtxSchedulerClient,
    state_reader: PtxStateReaderClient,
    latest_writes: HashMap<StateKey, TxnIdx>,
    num_txns: usize,
}

impl Worker {
    fn new(scheduler: PtxSchedulerClient, state_reader: PtxStateReaderClient) -> Self {
        Self {
            scheduler,
            state_reader,
            latest_writes: HashMap::with_capacity(EXPECTANT_BLOCK_KEYS),
            num_txns: 0,
        }
    }

    fn add_analyzed_transaction(&mut self, txn: AnalyzedTransaction) {
        let txn_idx = self.num_txns;
        self.num_txns += 1;

        // TODO(ptx): Reorder Non-P-Transactions. (Now we assume all are P-Txns.)
        let (txn, reads, read_writes) = txn.expect_p_txn();
        let mut dependencies = HashSet::new();
        self.process_txn_dependencies(
            txn_idx,
            reads,
            false, /* is_write_set */
            &mut dependencies,
        );
        self.process_txn_dependencies(
            txn_idx,
            read_writes,
            true, /* is_write_set */
            &mut dependencies,
        );

        self.scheduler.add_transaction(txn_idx, txn, dependencies);
    }

    fn process_txn_dependencies(
        &mut self,
        txn_idx: TxnIdx,
        keys: Vec<StateKey>,
        is_write_set: bool,
        dependencies: &mut HashSet<VersionedKey>,
    ) {
        for key in keys {
            match self.latest_writes.entry(key.clone()) {
                Entry::Occupied(mut entry) => {
                    dependencies.insert((key.clone(), *entry.get()));
                    if is_write_set {
                        *entry.get_mut() = txn_idx;
                    }
                },
                Entry::Vacant(entry) => {
                    dependencies.insert((key.clone(), BASE_VERSION));

                    // TODO(ptx): maybe prioritize reads that unblocks execution immediately.
                    self.state_reader.schedule_read(key);

                    if is_write_set {
                        entry.insert(txn_idx);
                    } else {
                        entry.insert(BASE_VERSION);
                    }
                }, // end Entry::Vacant
            } // end match
        } // end for
    }

    fn finish_block(&self) {
        self.scheduler.finish_block();
        self.state_reader.finish_block();
    }
}
