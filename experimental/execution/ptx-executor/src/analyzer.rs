// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! TODO(aldenhu): doc

use crate::{metrics::TIMER, sorter::PtxSorterClient};
use velor_logger::trace;
use velor_metrics_core::TimerHelper;
use velor_types::transaction::signature_verified_transaction::SignatureVerifiedTransaction;
use rayon::Scope;
use std::sync::mpsc::{channel, Sender};

pub(crate) struct PtxAnalyzer;

impl PtxAnalyzer {
    pub fn spawn(scope: &Scope, sorter: PtxSorterClient) -> PtxAnalyzerClient {
        let (work_tx, work_rx) = channel();
        scope.spawn(move |_scope| {
            let _timer = TIMER.timer_with(&["analyzer_block_total"]);
            loop {
                match work_rx.recv().expect("Channel closed.") {
                    Command::AnalyzeTransaction(txn) => {
                        let analyzed_txn = txn.into();
                        sorter.add_analyzed_transaction(analyzed_txn)
                    },
                    Command::Finish => {
                        sorter.finish_block();
                        trace!("finish_block.");
                        break;
                    },
                }
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
    pub fn analyze_transaction(&self, txn: SignatureVerifiedTransaction) {
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
    AnalyzeTransaction(SignatureVerifiedTransaction),
    Finish,
}
