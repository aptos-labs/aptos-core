// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{common::TxnIdx, state_view::OverlayedStateView};
use aptos_logger::trace;
use aptos_state_view::StateView;
use aptos_types::{transaction::TransactionOutput, write_set::TransactionWrite};
use aptos_vm_types::output::VMOutput;
use rayon::Scope;
use std::{
    collections::VecDeque,
    sync::mpsc::{channel, Sender},
};

pub(crate) struct PtxFinalizer;

impl PtxFinalizer {
    pub fn spawn<'scope, 'view: 'scope>(
        scope: &Scope<'scope>,
        base_view: &'view (dyn StateView + Sync),
        result_tx: Sender<TransactionOutput>,
    ) -> PtxFinalizerClient {
        let (work_tx, work_rx) = channel();
        scope.spawn(move |_scope| {
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
    vm_output_buffer: VecDeque<Option<VMOutput>>,
    next_idx: TxnIdx,
    state_view: OverlayedStateView<'view>,
}

impl<'view> Worker<'view> {
    fn new(base_view: &'view (dyn StateView + Sync), result_tx: Sender<TransactionOutput>) -> Self {
        Self {
            result_tx,
            vm_output_buffer: VecDeque::new(),
            next_idx: 0,
            state_view: OverlayedStateView::new(base_view),
        }
    }

    fn add_vm_output(&mut self, txn_idx: TxnIdx, vm_output: VMOutput) {
        trace!("seen txn: {}", txn_idx);
        assert!(txn_idx >= self.next_idx);
        let idx_in_buffer = txn_idx - self.next_idx;
        if self.vm_output_buffer.len() < idx_in_buffer + 1 {
            self.vm_output_buffer.resize(idx_in_buffer + 1, None);
        }
        self.vm_output_buffer[idx_in_buffer] = Some(vm_output);
        while self.ready_to_finalize_one() {
            trace!(
                "finalize txn: {}, buf len {}",
                txn_idx,
                self.vm_output_buffer.len(),
            );
            let vm_output = self.vm_output_buffer.pop_front().unwrap().unwrap();
            self.next_idx += 1;
            self.finalize_one(vm_output);
        }
    }

    fn ready_to_finalize_one(&self) -> bool {
        self.vm_output_buffer
            .front()
            .and_then(Option::as_ref)
            .is_some()
    }

    fn finalize_one(&mut self, vm_output: VMOutput) {
        let txn_out = vm_output
            .try_into_transaction_output(&self.state_view)
            .unwrap();
        for (key, op) in txn_out.write_set() {
            self.state_view.overwrite(key.clone(), op.as_state_value());
        }
        self.result_tx.send(txn_out).expect("Channel closed.");
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
