// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! TODO(aldenhu): doc

use crate::scheduler::PtxSchedulerClient;
use aptos_types::transaction::Transaction;
use rayon::Scope;
use std::sync::mpsc::{channel, Sender};

pub(crate) struct PtxAnalyzer;

impl PtxAnalyzer {
    pub fn spawn(scope: &Scope, scheduler: PtxSchedulerClient) -> PtxAnalyzerClient {
        let (work_tx, work_rx) = channel();
        scope.spawn(move |_scope| loop {
            match work_rx.recv().expect("Channel closed.") {
                Command::AnalyzeTransaction(txn) => {
                    let analyzed_txn = txn.into();
                    scheduler.add_analyzed_transaction(analyzed_txn)
                },
                Command::Finish => {
                    scheduler.finish_block();
                    break;
                },
            }
        });
        PtxAnalyzerClient { work_tx }
    }
}

#[derive(Clone)]
pub(crate) struct PtxAnalyzerClient {
    work_tx: Sender<Command>,
}

impl PtxAnalyzerClient {
    pub fn analyze_transaction(&self, txn: Transaction) {
        self.send_to_worker(Command::AnalyzeTransaction(txn));
    }

    pub fn finish_block(&self) {
        self.send_to_worker(Command::Finish);
    }

    fn send_to_worker(&self, command: Command) {
        self.work_tx.send(command).expect("Work thread died.");
    }
}

enum Command {
    AnalyzeTransaction(Transaction),
    Finish,
}
