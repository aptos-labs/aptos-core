// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! TODO(aldenhu): doc

use crate::{
    common::{HashMap, TxnIdx},
    finalizer::PtxFinalizerClient,
    metrics::{PER_WORKER_TIMER, TIMER},
    scheduler::PtxSchedulerClient,
    state_view::OverlayedStateView,
};
use aptos_logger::trace;
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue, StateView},
    transaction::{signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo},
    write_set::TransactionWrite,
};
use aptos_vm::AptosVM;
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::module_and_script_storage::AsAptosCodeStorage;
use rayon::Scope;
use std::sync::mpsc::{channel, Receiver, Sender};

pub(crate) struct PtxRunner;

impl PtxRunner {
    pub fn spawn<'scope, 'view: 'scope>(
        scope: &Scope<'scope>,
        base_view: &'view (impl StateView + Sync),
        finalizer: PtxFinalizerClient,
    ) -> PtxRunnerClient {
        let (work_tx, work_rx) = channel();
        scope.spawn(move |scope| {
            let _timer = TIMER.timer_with(&["runner_block_total"]);

            let first_command = work_rx.recv().expect("Channel closed.");
            let mut manager = match first_command {
                Command::SpawnWorkers {
                    scheduler,
                    num_workers,
                } => WorkerManager::new(scope, num_workers, finalizer, base_view, scheduler),
                _ => panic!("First command must be SpawnWorkers."),
            };

            loop {
                match work_rx.recv().expect("Work thread died.") {
                    Command::AddTransaction {
                        txn_idx,
                        transaction,
                        dependencies,
                    } => {
                        manager.add_transaction(txn_idx, transaction, dependencies);
                    },
                    Command::FinishBlock => {
                        manager.finish_block();
                        trace!("finish_block.");
                        break;
                    },
                    Command::SpawnWorkers { .. } => panic!("SpawnWorkers called twice."),
                }
            }
        });
        PtxRunnerClient { work_tx }
    }
}

enum Command {
    SpawnWorkers {
        scheduler: PtxSchedulerClient,
        num_workers: usize,
    },
    AddTransaction {
        txn_idx: TxnIdx,
        transaction: SignatureVerifiedTransaction,
        dependencies: HashMap<StateKey, Option<StateValue>>,
    },
    FinishBlock,
}

#[derive(Clone)]
pub(crate) struct PtxRunnerClient {
    work_tx: Sender<Command>,
}

impl PtxRunnerClient {
    pub fn spawn_workers(&self, scheduler: PtxSchedulerClient, num_workers: usize) {
        self.send_to_manager(Command::SpawnWorkers {
            scheduler,
            num_workers,
        })
    }

    pub fn add_transaction(
        &self,
        txn_idx: TxnIdx,
        transaction: SignatureVerifiedTransaction,
        dependencies: HashMap<StateKey, Option<StateValue>>,
    ) {
        self.send_to_manager(Command::AddTransaction {
            txn_idx,
            transaction,
            dependencies,
        })
    }

    /// Called by upstream to indicate all transactions form the block are added
    pub fn finish_block(&self) {
        self.send_to_manager(Command::FinishBlock)
    }

    fn send_to_manager(&self, command: Command) {
        self.work_tx.send(command).expect("Manager died.")
    }
}

type WorkerIndex = usize;

struct WorkerManager {
    finalizer: PtxFinalizerClient,
    work_txs: Vec<Sender<WorkerCommand>>,
    worker_ready_rx: Receiver<WorkerIndex>,
}

impl WorkerManager {
    fn new<'scope, 'view: 'scope, BaseView: StateView + Sync>(
        scope: &Scope<'scope>,
        num_workers: usize,
        finalizer: PtxFinalizerClient,
        base_view: &'view BaseView,
        scheduler: PtxSchedulerClient,
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
                    scheduler.clone(),
                )
            })
            .collect();

        Self {
            finalizer,
            work_txs,
            worker_ready_rx,
        }
    }

    fn add_transaction(
        &mut self,
        txn_idx: TxnIdx,
        transaction: SignatureVerifiedTransaction,
        dependencies: HashMap<StateKey, Option<StateValue>>,
    ) {
        // wait for next available worker
        let worker_index = {
            let _timer = TIMER.timer_with(&["runner_wait_ready_worker"]);
            self.worker_ready_rx.recv().expect("Channel closed.")
        };
        // send transaction to worker for execution
        self.work_txs[worker_index]
            .send(WorkerCommand::ExecuteTransaction {
                txn_idx,
                transaction,
                dependencies,
            })
            .expect("Worker died.");
    }

    fn finish_block(&mut self) {
        for work_tx in &self.work_txs {
            work_tx.send(WorkerCommand::Finish).expect("Worker died.")
        }
        // Wait for all workers to quit (all txns finished).
        while self.worker_ready_rx.recv().is_ok() {}

        // Inform the finalizer the that block is finished, after all work has surely finished.
        self.finalizer.finish_block();
    }
}

enum WorkerCommand {
    ExecuteTransaction {
        txn_idx: TxnIdx,
        transaction: SignatureVerifiedTransaction,
        dependencies: HashMap<StateKey, Option<StateValue>>,
    },
    Finish,
}

struct Worker<'view, BaseView> {
    finalizer: PtxFinalizerClient,
    work_rx: Receiver<WorkerCommand>,
    worker_ready_tx: Sender<WorkerIndex>,
    worker_index: WorkerIndex,
    base_view: &'view BaseView,
    scheduler: PtxSchedulerClient,
}

impl<'scope, 'view: 'scope, BaseView: StateView + Sync> Worker<'view, BaseView> {
    fn spawn(
        scope: &Scope<'scope>,
        finalizer: PtxFinalizerClient,
        worker_ready_tx: Sender<WorkerIndex>,
        worker_index: WorkerIndex,
        base_view: &'view BaseView,
        scheduler: PtxSchedulerClient,
    ) -> Sender<WorkerCommand> {
        let (work_tx, work_rx) = channel();
        scope.spawn(move |_scope| {
            let worker = Self {
                finalizer,
                work_rx,
                worker_ready_tx,
                worker_index,
                base_view,
                scheduler,
            };
            worker.work()
        });
        work_tx
    }

    fn work(self) {
        let idx = format!("{}", self.worker_index);
        let _timer = PER_WORKER_TIMER.timer_with(&[&idx, "block_total"]);
        // Share a VM in the same thread.
        // TODO(ptx): maybe warm up vm like done in AptosExecutorTask
        let env = AptosEnvironment::new(&self.base_view);
        let vm = {
            let _timer = PER_WORKER_TIMER.timer_with(&[&idx, "vm_init"]);
            AptosVM::new(&env, &self.base_view)
        };

        loop {
            self.worker_ready_tx
                .send(self.worker_index)
                .expect("Manager died.");

            let cmd = {
                let _timer = PER_WORKER_TIMER.timer_with(&[&idx, "wait_work"]);
                self.work_rx.recv().expect("Manager died.")
            };
            match cmd {
                WorkerCommand::ExecuteTransaction {
                    txn_idx,
                    transaction,
                    dependencies,
                } => {
                    let _total = PER_WORKER_TIMER.timer_with(&[&idx, "run_txn_total_with_drops"]);
                    let _total1 = PER_WORKER_TIMER.timer_with(&[&idx, "run_txn_total"]);
                    trace!("worker {} gonna run txn {}", self.worker_index, txn_idx);
                    let state_view =
                        OverlayedStateView::new_with_overlay(self.base_view, dependencies);
                    let log_context = AdapterLogSchema::new(self.base_view.id(), txn_idx);

                    let code_storage = state_view.as_aptos_code_storage(&env);
                    let vm_output = {
                        let _vm = PER_WORKER_TIMER.timer_with(&[&idx, "run_txn_vm"]);
                        vm.execute_single_transaction(
                            &transaction,
                            &vm.as_move_resolver(&state_view),
                            &code_storage,
                            &log_context,
                            &AuxiliaryInfo::default(),
                        )
                    };
                    let _post = PER_WORKER_TIMER.timer_with(&[&idx, "run_txn_post_vm"]);
                    // TODO(ptx): error handling
                    let (_vm_status, vm_output) = vm_output.expect("VM execution failed.");

                    // inform output state values to the manager
                    // TODO use try_into_storage_change_set() instead, and ChangeSet it returns, instead of VMOutput.
                    for (key, op) in vm_output.concrete_write_set_iter() {
                        self.scheduler.try_inform_state_value(
                            (key.clone(), txn_idx),
                            op.expect("PTX executor currently doesn't support non-concrete writes")
                                .as_state_value(),
                        );
                    }

                    self.finalizer.add_vm_output(txn_idx, vm_output);
                    trace!("worker {} finished txn {}", self.worker_index, txn_idx);
                    drop(_total1);
                },
                WorkerCommand::Finish => {
                    trace!("worker {} exit.", self.worker_index);
                    break;
                },
            } // end match command
        } // end loop
    } // end work
}
