// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    common::{TxnIdx, EXPECTANT_BLOCK_SIZE},
    metrics::TIMER,
    state_view::OverlayedStateView,
};
use aptos_logger::trace;
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    state_store::{state_key::StateKey, StateView},
    transaction::TransactionOutput,
    write_set::TransactionWrite,
};
use aptos_vm_types::output::VMOutput;
use once_cell::sync::Lazy;
use rayon::Scope;
use std::{
    collections::VecDeque,
    sync::mpsc::{channel, Sender},
};

pub static TOTAL_SUPPLY_STATE_KEY: Lazy<StateKey> = Lazy::new(|| {
    StateKey::table_item(
        &"1b854694ae746cdbd8d44186ca4929b2b337df21d1c74633be19b2710552fdca"
            .parse()
            .unwrap(),
        &[
            6, 25, 220, 41, 160, 170, 200, 250, 20, 103, 20, 5, 142, 141, 214, 210, 208, 243, 189,
            245, 246, 51, 25, 7, 191, 145, 243, 172, 216, 30, 105, 53,
        ],
    )
});

pub(crate) struct PtxFinalizer;

impl PtxFinalizer {
    pub fn spawn<'scope, 'view: 'scope>(
        scope: &Scope<'scope>,
        base_view: &'view (dyn StateView + Sync),
        result_tx: Sender<TransactionOutput>,
    ) -> PtxFinalizerClient {
        let (work_tx, work_rx) = channel();
        scope.spawn(move |_scope| {
            let _timer = TIMER.timer_with(&["finalizer_block_total"]);
            let mut worker = Worker::new(base_view, result_tx);
            loop {
                match work_rx.recv().expect("Channel closed.") {
                    Command::AddVMOutput { txn_idx, vm_output } => {
                        worker.add_vm_output(txn_idx, vm_output)
                    },
                    Command::FinishBlock => {
                        worker.finish_block();
                        break;
                    },
                }
            }
        });
        PtxFinalizerClient { work_tx }
    }
}

#[derive(Clone)]
pub(crate) struct PtxFinalizerClient {
    work_tx: Sender<Command>,
}

impl PtxFinalizerClient {
    pub fn add_vm_output(&self, txn_idx: TxnIdx, vm_output: VMOutput) {
        self.send_to_worker(Command::AddVMOutput { txn_idx, vm_output })
    }

    pub fn finish_block(&self) {
        self.send_to_worker(Command::FinishBlock)
    }

    fn send_to_worker(&self, command: Command) {
        self.work_tx.send(command).expect("Work thread died.");
    }
}

struct Worker<'view> {
    result_tx: Sender<TransactionOutput>,
    buffer: VecDeque<Option<VMOutput>>,
    next_idx: TxnIdx,
    state_view: OverlayedStateView<'view>,
}

impl<'view> Worker<'view> {
    fn new(base_view: &'view (dyn StateView + Sync), result_tx: Sender<TransactionOutput>) -> Self {
        Self {
            result_tx,
            buffer: VecDeque::with_capacity(EXPECTANT_BLOCK_SIZE),
            next_idx: 0,
            state_view: OverlayedStateView::new(base_view),
        }
    }

    fn add_vm_output(&mut self, txn_idx: TxnIdx, vm_output: VMOutput) {
        trace!("seen txn: {}", txn_idx);
        assert!(txn_idx >= self.next_idx);
        let idx_in_buffer = txn_idx - self.next_idx;
        if self.buffer.len() < idx_in_buffer + 1 {
            self.buffer.resize(idx_in_buffer + 1, None);
        }
        self.buffer[idx_in_buffer] = Some(vm_output);
        while self.ready_to_finalize_one() {
            trace!("finalize {}, buf len {}", txn_idx, self.buffer.len());
            self.finalize_one();
        }
    }

    fn ready_to_finalize_one(&self) -> bool {
        self.buffer.front().and_then(Option::as_ref).is_some()
    }

    fn finalize_one(&mut self) {
        let vm_output = self.buffer.pop_front().unwrap().unwrap();
        let txn_out = vm_output
            .try_materialize_into_transaction_output(&self.state_view)
            .unwrap();
        for (key, op) in txn_out.write_set().expect_write_op_iter() {
            // TODO(ptx): hack: deal only with the total supply
            if key == Lazy::force(&TOTAL_SUPPLY_STATE_KEY) {
                self.state_view.overwrite(key.clone(), op.as_state_value());
            }
        }
        self.result_tx.send(txn_out).expect("Channel closed.");
        self.next_idx += 1;
    }

    fn finish_block(&mut self) {
        trace!("finalizer: finish block at {}", self.next_idx);
    }
}

enum Command {
    AddVMOutput {
        txn_idx: TxnIdx,
        vm_output: VMOutput,
    },
    FinishBlock,
}
