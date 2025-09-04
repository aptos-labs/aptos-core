// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! TODO(aldenhu): doc

use crate::{
    common::{
        Entry, HashMap, HashSet, TxnIdx, VersionedKey, VersionedKeyHelper, EXPECTANT_BLOCK_SIZE,
    },
    metrics::TIMER,
    runner::PtxRunnerClient,
};
use velor_logger::trace;
use velor_metrics_core::TimerHelper;
use velor_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::signature_verified_transaction::SignatureVerifiedTransaction,
};
use rayon::Scope;
use std::sync::mpsc::{channel, Sender};

pub(crate) struct PtxScheduler;

impl PtxScheduler {
    pub fn spawn(scope: &Scope, runner: PtxRunnerClient) -> PtxSchedulerClient {
        let (work_tx, work_rx) = channel();
        let client = PtxSchedulerClient { work_tx };
        let client_clone = client.clone();
        scope.spawn(move |_scope| {
            let _timer = TIMER.timer_with(&["scheduler_block_total"]);
            let mut worker = Worker::new(runner, client_clone);
            loop {
                match work_rx.recv().expect("Work thread died.") {
                    Command::InformStateValue { key, value } => {
                        worker.inform_state_value(key, value);
                    },
                    Command::AddTransaction {
                        txn_idx,
                        transaction,
                        dependencies,
                    } => {
                        worker.add_transaction(txn_idx, transaction, dependencies);
                    },
                    Command::FinishBlock => {
                        trace!("finish_block.");
                        worker.finish_block();
                    },
                    Command::Exit => {
                        trace!("exit.");
                        break;
                    },
                }
            }
        });
        client
    }
}

#[derive(Clone)]
pub(crate) struct PtxSchedulerClient {
    work_tx: Sender<Command>,
}

impl PtxSchedulerClient {
    pub fn inform_state_value(&self, key: VersionedKey, value: Option<StateValue>) {
        trace!("inform_val {}", key.1);
        self.send_to_worker(Command::InformStateValue { key, value });
    }

    pub fn try_inform_state_value(&self, key: VersionedKey, value: Option<StateValue>) {
        // TODO(aldenhu): hack: scheduler quits before runner
        self.work_tx
            .send(Command::InformStateValue { key, value })
            .ok();
    }

    pub fn add_transaction(
        &self,
        txn_idx: TxnIdx,
        transaction: SignatureVerifiedTransaction,
        dependencies: HashSet<(StateKey, TxnIdx)>,
    ) {
        trace!("add_txn {}", txn_idx);
        self.send_to_worker(Command::AddTransaction {
            txn_idx,
            transaction,
            dependencies,
        });
    }

    pub fn finish_block(&self) {
        trace!("finish_block.");
        self.send_to_worker(Command::FinishBlock);
    }

    fn exit(&self) {
        self.send_to_worker(Command::Exit);
    }

    fn send_to_worker(&self, command: Command) {
        self.work_tx.send(command).expect("Work thread died.");
    }
}

enum Command {
    InformStateValue {
        key: VersionedKey,
        value: Option<StateValue>,
    },
    AddTransaction {
        txn_idx: TxnIdx,
        transaction: SignatureVerifiedTransaction,
        dependencies: HashSet<(StateKey, TxnIdx)>,
    },
    FinishBlock,
    Exit,
}

enum StateValueState {
    Pending { subscribers: Vec<TxnIdx> },
    Ready { value: Option<StateValue> },
}

struct PendingTransaction {
    transaction: SignatureVerifiedTransaction,
    pending_dependencies: HashSet<VersionedKey>,
    met_dependencies: HashMap<StateKey, Option<StateValue>>,
}

impl PendingTransaction {
    fn new(transaction: SignatureVerifiedTransaction) -> Self {
        Self {
            transaction,
            pending_dependencies: HashSet::new(),
            met_dependencies: HashMap::new(),
        }
    }
}

struct Worker {
    transactions: Vec<Option<PendingTransaction>>,
    state_values: Vec<HashMap<StateKey, StateValueState>>,
    num_pending_txns: usize,
    block_finished: bool,
    runner: PtxRunnerClient,
    myself: PtxSchedulerClient,
}

impl Worker {
    fn new(runner: PtxRunnerClient, myself: PtxSchedulerClient) -> Self {
        let mut state_values = Vec::with_capacity(EXPECTANT_BLOCK_SIZE);
        state_values.push(HashMap::new()); // for base reads
        Self {
            transactions: Vec::with_capacity(EXPECTANT_BLOCK_SIZE),
            state_values,
            num_pending_txns: 0,
            block_finished: false,
            runner,
            myself,
        }
    }

    pub fn inform_state_value(&mut self, versioned_key: VersionedKey, value: Option<StateValue>) {
        let _timer = TIMER.timer_with(&["scheduler_inform_state_value"]);
        match self.state_values[versioned_key.txn_idx_shifted()].entry(versioned_key.key().clone())
        {
            Entry::Occupied(mut existing) => {
                let old_state = existing.insert(StateValueState::Ready {
                    value: value.clone(),
                });
                match old_state {
                    StateValueState::Pending { subscribers } => {
                        for subscriber in subscribers {
                            // TODO(ptx): reduce clone / memcpy
                            self.inform_state_value_to_txn(
                                subscriber,
                                versioned_key.clone(),
                                value.clone(),
                            );
                        }
                    },
                    StateValueState::Ready { .. } => {
                        unreachable!("StateValue pushed twice.")
                    },
                }
            },
            Entry::Vacant(vacant) => {
                vacant.insert(StateValueState::Ready { value });
            },
        }
    }

    fn inform_state_value_to_txn(
        &mut self,
        txn_idx: TxnIdx,
        versioned_key: VersionedKey,
        value: Option<StateValue>,
    ) {
        let pending_txn = self.transactions[txn_idx].as_mut().unwrap();
        let found_dependency = pending_txn.pending_dependencies.remove(&versioned_key);
        assert!(found_dependency, "Pending dependency not found.");
        let (key, _txn_idx) = versioned_key;
        pending_txn.met_dependencies.insert(key, value);

        if pending_txn.pending_dependencies.is_empty() {
            self.run_transaction(txn_idx);
        }
    }

    fn take_pending_txn(&mut self, txn_idx: TxnIdx) -> PendingTransaction {
        self.num_pending_txns -= 1;
        trace!("pending txns: {}", self.num_pending_txns);
        self.transactions[txn_idx]
            .take()
            .expect("Transaction is not Pending.")
    }

    pub fn add_transaction(
        &mut self,
        txn_idx: TxnIdx,
        transaction: SignatureVerifiedTransaction,
        dependencies: HashSet<VersionedKey>,
    ) {
        let _timer = TIMER.timer_with(&["scheduler_add_txn"]);
        assert_eq!(txn_idx, self.transactions.len());
        self.transactions
            .push(Some(PendingTransaction::new(transaction)));
        self.state_values.push(HashMap::new());
        self.num_pending_txns += 1;
        let pending_txn = self.transactions[txn_idx].as_mut().unwrap();

        for versioned_key in dependencies {
            match self.state_values[versioned_key.txn_idx_shifted()]
                .entry(versioned_key.key().clone())
            {
                Entry::Occupied(mut existing) => match existing.get_mut() {
                    StateValueState::Pending { subscribers } => {
                        pending_txn.pending_dependencies.insert(versioned_key);
                        subscribers.push(txn_idx);
                    },
                    StateValueState::Ready { value } => {
                        pending_txn
                            .met_dependencies
                            .insert(versioned_key.key().clone(), value.clone());
                    },
                },
                Entry::Vacant(vacant) => {
                    pending_txn.pending_dependencies.insert(versioned_key);
                    vacant.insert(StateValueState::Pending {
                        subscribers: vec![txn_idx],
                    });
                },
            }
        }

        if pending_txn.pending_dependencies.is_empty() {
            self.run_transaction(txn_idx);
        }
    }

    pub fn finish_block(&mut self) {
        self.block_finished = true;
        self.maybe_exit();
    }

    fn maybe_exit(&self) {
        if self.block_finished && self.num_pending_txns == 0 {
            self.runner.finish_block();
            self.myself.exit();
        }
    }

    fn run_transaction(&mut self, txn_idx: TxnIdx) {
        let PendingTransaction {
            transaction,
            pending_dependencies,
            met_dependencies,
        } = self.take_pending_txn(txn_idx);
        assert!(
            pending_dependencies.is_empty(),
            "Transaction has pending dependencies."
        );

        self.runner
            .add_transaction(txn_idx, transaction, met_dependencies);
        // Try to exit only after the sending the work to the runner, so that a
        // `runner.finish_block()` call happens after that.
        self.maybe_exit();
    }
}
