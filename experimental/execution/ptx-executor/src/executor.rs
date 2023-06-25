// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! TODO(aldenhu): doc

use crate::{
    common::{TxnIdx, VersionedKey, EXPECTANT_BLOCK_KEYS, EXPECTANT_BLOCK_SIZE},
    finalizer::PtxFinalizerClient,
    state_view::OverlayedStateView,
};
use aptos_state_view::StateView;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Transaction,
    write_set::TransactionWrite,
};
use aptos_vm::{
    adapter_common::{preprocess_transaction, VMAdapter},
    data_cache::AsMoveResolver,
    AptosVM,
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use rayon::Scope;
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    sync::mpsc::{channel, Receiver, Sender},
};

pub(crate) struct PtxExecutor;

impl PtxExecutor {
    pub fn spawn<'scope, 'view: 'scope>(
        scope: &Scope<'scope>,
        base_view: &'view (impl StateView + Sync),
        num_workers: usize,
        finalizer: PtxFinalizerClient,
    ) -> PtxExecutorClient {
        let (work_tx, work_rx) = channel();
        let client = PtxExecutorClient { work_tx };
        let mut manager =
            WorkerManager::new(scope, num_workers, finalizer, base_view, client.clone());
        scope.spawn(move |_scope| loop {
            match work_rx.recv().expect("Work thread died.") {
                Command::InformStateValue { key, value } => {
                    manager.inform_state_value(key, value);
                },
                Command::AddTransaction {
                    transaction,
                    dependencies,
                } => {
                    manager.add_transaction(transaction, dependencies);
                },
                Command::FinishBlock => {
                    manager.finish_block();
                },
                Command::Exit => {
                    break;
                },
            }
        });
        client
    }
}

#[derive(Clone)]
pub(crate) struct PtxExecutorClient {
    work_tx: Sender<Command>,
}

impl PtxExecutorClient {
    pub fn inform_state_value(&self, key: VersionedKey, value: Option<StateValue>) {
        self.send_to_manager(Command::InformStateValue { key, value })
    }

    fn send_to_manager(&self, command: Command) {
        self.work_tx.send(command).expect("Channel died.")
    }

    pub fn add_transaction(
        &self,
        transaction: Transaction,
        dependencies: HashSet<(StateKey, TxnIdx)>,
    ) {
        self.send_to_manager(Command::AddTransaction {
            transaction,
            dependencies,
        })
    }

    /// Called by upstream to indicate all transactions form the block are added
    pub fn finish_block(&self) {
        self.send_to_manager(Command::FinishBlock)
    }

    /// Called internally to inform the outer loop to exit
    fn exit(&self) {
        self.send_to_manager(Command::Exit)
    }
}

enum Command {
    InformStateValue {
        key: VersionedKey,
        value: Option<StateValue>,
    },
    AddTransaction {
        transaction: Transaction,
        dependencies: HashSet<(StateKey, TxnIdx)>,
    },
    FinishBlock,
    Exit,
}

type WorkerIndex = usize;

enum StateValueState {
    Pending { subscribers: Vec<TxnIdx> },
    Ready { value: Option<StateValue> },
}

struct PendingTransaction {
    transaction: Transaction,
    pending_dependencies: HashSet<VersionedKey>,
    met_dependencies: HashMap<StateKey, Option<StateValue>>,
}

impl PendingTransaction {
    fn new(transaction: Transaction) -> Self {
        Self {
            transaction,
            pending_dependencies: HashSet::new(),
            met_dependencies: HashMap::new(),
        }
    }
}

struct WorkerManager {
    finalizer: PtxFinalizerClient,
    work_txs: Vec<Sender<WorkerCommand>>,
    worker_ready_rx: Receiver<WorkerIndex>,
    transactions: Vec<Option<PendingTransaction>>,
    state_values: HashMap<VersionedKey, StateValueState>,
    num_pending_txns: usize,
    block_finished: bool,
    executor: PtxExecutorClient,
}

impl WorkerManager {
    fn new<'scope, 'view: 'scope, BaseView: StateView + Sync>(
        scope: &Scope<'scope>,
        num_workers: usize,
        finalizer: PtxFinalizerClient,
        base_view: &'view BaseView,
        executor: PtxExecutorClient,
    ) -> Self {
        let (worker_ready_tx, worker_ready_rx) = channel();
        let work_txs = (0..num_workers)
            .map(|worker_idx| {
                Worker::<BaseView>::spawn(
                    scope,
                    finalizer.clone(),
                    worker_ready_tx.clone(),
                    worker_idx,
                    base_view,
                    executor.clone(),
                )
            })
            .collect();

        Self {
            finalizer,
            work_txs,
            worker_ready_rx,
            executor,
            transactions: Vec::with_capacity(EXPECTANT_BLOCK_SIZE),
            state_values: HashMap::with_capacity(EXPECTANT_BLOCK_KEYS),
            num_pending_txns: 0,
            block_finished: false,
        }
    }

    // Top level API
    fn inform_state_value(&mut self, key: VersionedKey, value: Option<StateValue>) {
        match self.state_values.entry(key.clone()) {
            Entry::Occupied(mut existing) => {
                let old_state = existing.insert(StateValueState::Ready {
                    value: value.clone(),
                });
                match old_state {
                    StateValueState::Pending { subscribers } => {
                        for subscriber in subscribers {
                            // TODO(ptx): reduce clone / memcpy
                            self.inform_state_value_to_txn(subscriber, key.clone(), value.clone());
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
        let pending_txn = self.borrow_pending_txn(txn_idx);
        let found_dependency = pending_txn.pending_dependencies.remove(&versioned_key);
        assert!(found_dependency, "Pending dependency not found.");
        let (key, _txn_idx) = versioned_key;
        pending_txn.met_dependencies.insert(key, value);

        if pending_txn.pending_dependencies.is_empty() {
            self.execute_transaction(txn_idx);
        }
    }

    fn borrow_pending_txn(&mut self, txn_idx: TxnIdx) -> &mut PendingTransaction {
        self.transactions[txn_idx]
            .as_mut()
            .expect("Transaction is not Pending.")
    }

    fn take_pending_txn(&mut self, txn_idx: TxnIdx) -> PendingTransaction {
        self.num_pending_txns -= 1;
        self.transactions[txn_idx]
            .take()
            .expect("Transaction is not Pending.")
    }

    // Top level API
    fn add_transaction(&mut self, transaction: Transaction, dependencies: HashSet<VersionedKey>) {
        let txn_idx = self.transactions.len();
        self.transactions
            .push(Some(PendingTransaction::new(transaction)));
        self.num_pending_txns += 1;
        let pending_txn = self.transactions[txn_idx].as_mut().unwrap();

        for versioned_key in dependencies {
            match self.state_values.entry(versioned_key.clone()) {
                Entry::Occupied(mut existing) => match existing.get_mut() {
                    StateValueState::Pending { subscribers } => {
                        pending_txn.pending_dependencies.insert(versioned_key);
                        subscribers.push(txn_idx);
                    },
                    StateValueState::Ready { value } => {
                        let (key, _txn_idx) = versioned_key;
                        pending_txn.met_dependencies.insert(key, value.clone());
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
            self.execute_transaction(txn_idx);
        }
    }

    fn execute_transaction(&mut self, txn_idx: TxnIdx) {
        let PendingTransaction {
            transaction,
            pending_dependencies,
            met_dependencies,
        } = self.take_pending_txn(txn_idx);
        assert!(
            pending_dependencies.is_empty(),
            "Transaction has pending dependencies."
        );

        // wait for next available worker
        let worker_index = self.worker_ready_rx.recv().expect("Channel closed.");
        // send transaction to worker for execution
        self.work_txs[worker_index]
            .send(WorkerCommand::ExecuteTransaction {
                txn_idx,
                transaction,
                met_dependencies,
            })
            .expect("Worker died.");
        // In case this is the last transaction, we exit. This needs to happen after the last piece
        // of work is sent to the worker
        self.maybe_exit()
    }

    fn finish_block(&mut self) {
        self.block_finished = true;
        self.maybe_exit();
    }

    fn maybe_exit(&self) {
        if self.block_finished && self.num_pending_txns == 0 {
            // Inform all workers to exit after finishing the last piece of work.
            for work_tx in &self.work_txs {
                work_tx.send(WorkerCommand::Finish).ok();
            }
            // Wait for all works to quit.
            while self.worker_ready_rx.recv().is_ok() {}

            // Inform the finalizer the that block is finished, after all work surely finishes.
            self.finalizer.finish_block();
            self.executor.exit();
        }
    }
}

enum WorkerCommand {
    ExecuteTransaction {
        txn_idx: TxnIdx,
        transaction: Transaction,
        met_dependencies: HashMap<StateKey, Option<StateValue>>,
    },
    Finish,
}

struct Worker<'view, BaseView> {
    finalizer: PtxFinalizerClient,
    work_rx: Receiver<WorkerCommand>,
    worker_ready_tx: Sender<WorkerIndex>,
    worker_index: WorkerIndex,
    base_view: &'view BaseView,
    executor: PtxExecutorClient,
}

impl<'scope, 'view: 'scope, BaseView: StateView + Sync> Worker<'view, BaseView> {
    fn spawn(
        scope: &Scope<'scope>,
        finalizer: PtxFinalizerClient,
        worker_ready_tx: Sender<WorkerIndex>,
        worker_index: WorkerIndex,
        base_view: &'view BaseView,
        executor: PtxExecutorClient,
    ) -> Sender<WorkerCommand> {
        let (work_tx, work_rx) = channel();
        scope.spawn(move |_scope| {
            let worker = Self {
                finalizer,
                work_rx,
                worker_ready_tx,
                worker_index,
                base_view,
                executor,
            };
            worker.work()
        });
        work_tx
    }

    fn work(self) {
        // Share a VM in the same thread.
        // TODO(ptx): maybe warm up vm like done in AptosExecutorTask
        let vm = AptosVM::new(&self.base_view.as_move_resolver());

        // Inform the manger worker is up.
        self.worker_ready_tx
            .send(self.worker_index)
            .expect("Manager died.");

        #[allow(clippy::while_let_loop)]
        loop {
            match self.work_rx.recv().expect("Sender died.") {
                WorkerCommand::ExecuteTransaction {
                    txn_idx,
                    transaction,
                    met_dependencies,
                } => {
                    let state_view =
                        OverlayedStateView::new_with_overlay(self.base_view, met_dependencies);
                    let log_context = AdapterLogSchema::new(self.base_view.id(), txn_idx);
                    // TODO(ptx): avoid cloning
                    let preprocessed_txn = preprocess_transaction::<AptosVM>(transaction);

                    let vm_output = vm.execute_single_transaction(
                        &preprocessed_txn,
                        &vm.as_move_resolver(&state_view),
                        &log_context,
                    );
                    // TODO(ptx): error handling
                    let (_vm_status, vm_output, _msg) = vm_output.expect("VM execution failed.");

                    // inform output state values to the manager
                    for (key, op) in vm_output.change_set().write_set_iter() {
                        self.executor
                            .inform_state_value((key.clone(), txn_idx), op.as_state_value());
                    }

                    self.finalizer.add_vm_output(txn_idx, vm_output);

                    // Inform the manager worker is ready for more work.
                    self.worker_ready_tx
                        .send(self.worker_index)
                        .expect("Manager died.");
                },
                WorkerCommand::Finish => {
                    break;
                },
            }
        }
    }
}
