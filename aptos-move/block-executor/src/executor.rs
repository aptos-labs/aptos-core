// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    counters::{
        GasType, PARALLEL_EXECUTION_SECONDS, RAYON_EXECUTION_SECONDS, TASK_EXECUTE_SECONDS,
        TASK_VALIDATE_SECONDS, VM_INIT_SECONDS, WORK_WITH_TASK_SECONDS,
    },
    errors::*,
    scheduler::{DependencyStatus, ExecutionTaskType, Scheduler, SchedulerTask, Wave},
    task::{ExecutionStatus, ExecutorTask, Transaction, TransactionOutput},
    txn_commit_hook::TransactionCommitHook,
    txn_last_input_output::TxnLastInputOutput,
    view::{LatestView, MVHashMapView},
};
use aptos_aggregator::delta_change_set::{deserialize, serialize};
use aptos_logger::{debug, info};
use aptos_mvhashmap::{
    types::{MVDataError, MVDataOutput, TxnIndex, Version},
    unsync_map::UnsyncMap,
    MVHashMap,
};
use aptos_state_view::TStateView;
use aptos_types::{
    block_executor::partitioner::BlockExecutorTransactions, executable::Executable,
    fee_statement::FeeStatement, write_set::WriteOp,
};
use aptos_vm_logging::{clear_speculative_txn_logs, init_speculative_logs};
use num_cpus;
use rayon::ThreadPool;
use std::{
    marker::PhantomData,
    sync::{
        mpsc,
        mpsc::{Receiver, Sender},
        Arc,
    },
};

struct CommitGuard<'a> {
    post_commit_txs: &'a Vec<Sender<u32>>,
    worker_idx: usize,
    txn_idx: u32,
}

impl<'a> CommitGuard<'a> {
    fn new(post_commit_txs: &'a Vec<Sender<u32>>, worker_idx: usize, txn_idx: u32) -> Self {
        Self {
            post_commit_txs,
            worker_idx,
            txn_idx,
        }
    }
}

impl<'a> Drop for CommitGuard<'a> {
    fn drop(&mut self) {
        // Send the committed txn to the Worker thread.
        self.post_commit_txs[self.worker_idx]
            .send(self.txn_idx)
            .expect("Worker must be available");
    }
}

#[derive(Debug)]
enum CommitRole {
    Coordinator(Vec<Sender<TxnIndex>>),
    Worker(Receiver<TxnIndex>),
}

pub struct BlockExecutor<T, E, S, L, X> {
    // number of active concurrent tasks, corresponding
    // to the maximum number of rayon
    // threads that may be concurrently participating in parallel execution.
    concurrency_level: usize,
    executor_thread_pool: Arc<ThreadPool>,
    maybe_block_gas_limit: Option<u64>,
    transaction_commit_hook: Option<L>,
    phantom: PhantomData<(T, E, S, L, X)>,
}

impl<T, E, S, L, X> BlockExecutor<T, E, S, L, X>
where
    T: Transaction,
    E: ExecutorTask<Txn = T>,
    S: TStateView<Key = T::Key> + Sync,
    L: TransactionCommitHook<Output = E::Output>,
    X: Executable + 'static,
{
    /// The caller needs to ensure that concurrency_level > 1 (0 is illegal and 1 should
    /// be handled by sequential execution) and that concurrency_level <= num_cpus.
    pub fn new(
        concurrency_level: usize,
        executor_thread_pool: Arc<ThreadPool>,
        maybe_block_gas_limit: Option<u64>,
        transaction_commit_hook: Option<L>,
    ) -> Self {
        assert!(
            concurrency_level > 0 && concurrency_level <= num_cpus::get(),
            "Parallel execution concurrency level {} should be between 1 and number of CPUs",
            concurrency_level
        );
        Self {
            concurrency_level,
            executor_thread_pool,
            maybe_block_gas_limit,
            transaction_commit_hook,
            phantom: PhantomData,
        }
    }

    fn update_parallel_block_gas_counters(
        accumulated_fee_statement: &FeeStatement,
        num_committed: usize,
    ) {
        counters::observe_parallel_execution_block_gas(
            accumulated_fee_statement.gas_used(),
            GasType::TOTAL_GAS,
        );
        counters::observe_parallel_execution_block_gas(
            accumulated_fee_statement.execution_gas_used(),
            GasType::EXECUTION_GAS,
        );
        counters::observe_parallel_execution_block_gas(
            accumulated_fee_statement.io_gas_used(),
            GasType::IO_GAS,
        );
        counters::observe_parallel_execution_block_gas(
            accumulated_fee_statement.storage_gas_used(),
            GasType::STORAGE_GAS,
        );
        counters::observe_parallel_execution_block_gas(
            accumulated_fee_statement.execution_gas_used()
                + accumulated_fee_statement.io_gas_used(),
            GasType::NON_STORAGE_GAS,
        );
        counters::observe_parallel_execution_block_gas(
            accumulated_fee_statement.storage_fee_used(),
            GasType::STORAGE_FEE,
        );
        counters::BLOCK_COMMITTED_TXNS
            .with_label_values(&[counters::Mode::PARALLEL])
            .observe(num_committed as f64);
    }

    fn update_parallel_txn_gas_counters(txn_fee_statements: &Vec<FeeStatement>) {
        for fee_statement in txn_fee_statements {
            counters::observe_parallel_execution_txn_gas(
                fee_statement.gas_used(),
                GasType::TOTAL_GAS,
            );
            counters::observe_parallel_execution_txn_gas(
                fee_statement.execution_gas_used(),
                GasType::EXECUTION_GAS,
            );
            counters::observe_parallel_execution_txn_gas(
                fee_statement.io_gas_used(),
                GasType::IO_GAS,
            );
            counters::observe_parallel_execution_txn_gas(
                fee_statement.storage_gas_used(),
                GasType::STORAGE_GAS,
            );
            counters::observe_parallel_execution_txn_gas(
                fee_statement.execution_gas_used() + fee_statement.io_gas_used(),
                GasType::NON_STORAGE_GAS,
            );
            counters::observe_parallel_execution_txn_gas(
                fee_statement.storage_fee_used(),
                GasType::STORAGE_FEE,
            );
        }
    }

    fn update_sequential_block_gas_counters(
        accumulated_fee_statement: &FeeStatement,
        num_committed: usize,
    ) {
        counters::observe_sequential_execution_block_gas(
            accumulated_fee_statement.gas_used(),
            GasType::TOTAL_GAS,
        );
        counters::observe_sequential_execution_block_gas(
            accumulated_fee_statement.execution_gas_used(),
            GasType::EXECUTION_GAS,
        );
        counters::observe_sequential_execution_block_gas(
            accumulated_fee_statement.io_gas_used(),
            GasType::IO_GAS,
        );
        counters::observe_sequential_execution_block_gas(
            accumulated_fee_statement.storage_gas_used(),
            GasType::STORAGE_GAS,
        );
        counters::observe_sequential_execution_block_gas(
            accumulated_fee_statement.execution_gas_used()
                + accumulated_fee_statement.io_gas_used(),
            GasType::NON_STORAGE_GAS,
        );
        counters::observe_sequential_execution_block_gas(
            accumulated_fee_statement.storage_fee_used(),
            GasType::STORAGE_FEE,
        );
        counters::BLOCK_COMMITTED_TXNS
            .with_label_values(&[counters::Mode::PARALLEL])
            .observe(num_committed as f64);
    }

    fn update_sequential_txn_gas_counters(fee_statement: &FeeStatement) {
        counters::observe_sequential_execution_txn_gas(
            fee_statement.gas_used(),
            GasType::TOTAL_GAS,
        );
        counters::observe_sequential_execution_txn_gas(
            fee_statement.execution_gas_used(),
            GasType::EXECUTION_GAS,
        );
        counters::observe_sequential_execution_txn_gas(
            fee_statement.io_gas_used(),
            GasType::IO_GAS,
        );
        counters::observe_sequential_execution_txn_gas(
            fee_statement.storage_gas_used(),
            GasType::STORAGE_GAS,
        );
        counters::observe_sequential_execution_txn_gas(
            fee_statement.execution_gas_used() + fee_statement.io_gas_used(),
            GasType::NON_STORAGE_GAS,
        );
        counters::observe_sequential_execution_txn_gas(
            fee_statement.storage_fee_used(),
            GasType::STORAGE_FEE,
        );
    }

    fn execute(
        &self,
        version: Version,
        signature_verified_block: &[T],
        last_input_output: &TxnLastInputOutput<T::Key, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Value, X>,
        scheduler: &Scheduler,
        executor: &E,
        base_view: &S,
    ) -> SchedulerTask {
        let _timer = TASK_EXECUTE_SECONDS.start_timer();
        let (idx_to_execute, incarnation) = version;
        let txn = &signature_verified_block[idx_to_execute as usize];

        let speculative_view = MVHashMapView::new(versioned_cache, scheduler);

        // VM execution.
        let execute_result = executor.execute_transaction(
            &LatestView::<T, S, X>::new_mv_view(base_view, &speculative_view, idx_to_execute),
            txn,
            idx_to_execute,
            false,
        );
        let mut prev_modified_keys = last_input_output.modified_keys(idx_to_execute);

        // For tracking whether the recent execution wrote outside of the previous write/delta set.
        let mut updates_outside = false;
        let mut apply_updates = |output: &E::Output| {
            // First, apply writes.
            let write_version = (idx_to_execute, incarnation);
            for (k, v) in output.get_writes().into_iter() {
                if !prev_modified_keys.remove(&k) {
                    updates_outside = true;
                }
                versioned_cache.write(k, write_version, v);
            }

            // Then, apply deltas.
            for (k, d) in output.get_deltas().into_iter() {
                if !prev_modified_keys.remove(&k) {
                    updates_outside = true;
                }
                versioned_cache.add_delta(k, idx_to_execute, d);
            }
        };

        let result = match execute_result {
            // These statuses are the results of speculative execution, so even for
            // SkipRest (skip the rest of transactions) and Abort (abort execution with
            // user defined error), no immediate action is taken. Instead the statuses
            // are recorded and (final statuses) are analyzed when the block is executed.
            ExecutionStatus::Success(output) => {
                // Apply the writes/deltas to the versioned_data_cache.
                apply_updates(&output);
                ExecutionStatus::Success(output)
            },
            ExecutionStatus::SkipRest(output) => {
                // Apply the writes/deltas and record status indicating skip.
                apply_updates(&output);
                ExecutionStatus::SkipRest(output)
            },
            ExecutionStatus::Abort(err) => {
                // Record the status indicating abort.
                ExecutionStatus::Abort(Error::UserError(err))
            },
        };

        // Remove entries from previous write/delta set that were not overwritten.
        for k in prev_modified_keys {
            versioned_cache.delete(&k, idx_to_execute);
        }

        if last_input_output
            .record(idx_to_execute, speculative_view.take_reads(), result)
            .is_err()
        {
            // When there is module publishing r/w intersection, can early halt BlockSTM to
            // fallback to sequential execution.
            scheduler.halt();
            return SchedulerTask::NoTask;
        }
        scheduler.finish_execution(idx_to_execute, incarnation, updates_outside)
    }

    fn validate(
        &self,
        version_to_validate: Version,
        validation_wave: Wave,
        last_input_output: &TxnLastInputOutput<T::Key, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Value, X>,
        scheduler: &Scheduler,
    ) -> SchedulerTask {
        use MVDataError::*;
        use MVDataOutput::*;

        let _timer = TASK_VALIDATE_SECONDS.start_timer();
        let (idx_to_validate, incarnation) = version_to_validate;
        let read_set = last_input_output
            .read_set(idx_to_validate)
            .expect("[BlockSTM]: Prior read-set must be recorded");

        let valid = read_set.iter().all(|r| {
            match versioned_cache.fetch_data(r.path(), idx_to_validate) {
                Ok(Versioned(version, _)) => r.validate_version(version),
                Ok(Resolved(value)) => r.validate_resolved(value),
                // Dependency implies a validation failure, and if the original read were to
                // observe an unresolved delta, it would set the aggregator base value in the
                // multi-versioned data-structure, resolve, and record the resolved value.
                Err(Dependency(_)) | Err(Unresolved(_)) => false,
                Err(NotFound) => r.validate_storage(),
                // We successfully validate when read (again) results in a delta application
                // failure. If the failure is speculative, a later validation will fail due to
                // a read without this error. However, if the failure is real, passing
                // validation here allows to avoid infinitely looping and instead panic when
                // materializing deltas as writes in the final output preparation state. Panic
                // is also preferable as it allows testing for this scenario.
                Err(DeltaApplicationFailure) => r.validate_delta_application_failure(),
            }
        });

        let aborted = !valid && scheduler.try_abort(idx_to_validate, incarnation);

        if aborted {
            counters::SPECULATIVE_ABORT_COUNT.inc();

            // Any logs from the aborted execution should be cleared and not reported.
            clear_speculative_txn_logs(idx_to_validate as usize);

            // Not valid and successfully aborted, mark the latest write/delta sets as estimates.
            for k in last_input_output.modified_keys(idx_to_validate) {
                versioned_cache.mark_estimate(&k, idx_to_validate);
            }

            scheduler.finish_abort(idx_to_validate, incarnation)
        } else {
            scheduler.finish_validation(idx_to_validate, validation_wave);
            SchedulerTask::NoTask
        }
    }

    fn coordinator_commit_hook(
        &self,
        maybe_block_gas_limit: Option<u64>,
        scheduler: &Scheduler,
        post_commit_txs: &Vec<Sender<u32>>,
        worker_idx: &mut usize,
        scheduler_task: &mut SchedulerTask,
        last_input_output: &TxnLastInputOutput<T::Key, E::Output, E::Error>,
        accumulated_fee_statement: &mut FeeStatement,
        txn_fee_statements: &mut Vec<FeeStatement>,
    ) {
        while let Some(txn_idx) = scheduler.try_commit() {
            // Create a CommitGuard to ensure Coordinator sends the committed txn index to Worker.
            let _commit_guard: CommitGuard =
                CommitGuard::new(post_commit_txs, *worker_idx, txn_idx);
            // Iterate round robin over workers to do commit_hook.
            *worker_idx = (*worker_idx + 1) % post_commit_txs.len();

            // Committed the last transaction, BlockSTM finishes execution.
            if txn_idx as usize + 1 == scheduler.num_txns() as usize {
                *scheduler_task = SchedulerTask::Done;

                Self::update_parallel_block_gas_counters(
                    accumulated_fee_statement,
                    (txn_idx + 1) as usize,
                );
                Self::update_parallel_txn_gas_counters(txn_fee_statements);
                info!(
                    "[BlockSTM]: Parallel execution completed, all {} txns committed.",
                    txn_idx + 1
                );
                break;
            }

            // For committed txns with Success status, calculate the accumulated gas costs.
            // For committed txns with Abort or SkipRest status, early halt BlockSTM.
            match last_input_output.fee_statement(txn_idx) {
                Some(fee_statement) => {
                    accumulated_fee_statement.add_fee_statement(&fee_statement);
                    txn_fee_statements.push(fee_statement);
                },
                None => {
                    scheduler.halt();

                    Self::update_parallel_block_gas_counters(
                        accumulated_fee_statement,
                        (txn_idx + 1) as usize,
                    );
                    Self::update_parallel_txn_gas_counters(txn_fee_statements);
                    info!("[BlockSTM]: Parallel execution early halted due to Abort or SkipRest txn, {} txns committed.", txn_idx + 1);
                    break;
                },
            };

            if let Some(per_block_gas_limit) = maybe_block_gas_limit {
                // When the accumulated execution and io gas of the committed txns exceeds PER_BLOCK_GAS_LIMIT, early halt BlockSTM.
                // Storage gas does not count towards the per block gas limit, as we measure execution related cost here.
                let accumulated_non_storage_gas = accumulated_fee_statement.execution_gas_used()
                    + accumulated_fee_statement.io_gas_used();
                if accumulated_non_storage_gas >= per_block_gas_limit {
                    // Set the execution output status to be SkipRest, to skip the rest of the txns.
                    last_input_output.update_to_skip_rest(txn_idx);
                    scheduler.halt();

                    Self::update_parallel_block_gas_counters(
                        accumulated_fee_statement,
                        (txn_idx + 1) as usize,
                    );
                    Self::update_parallel_txn_gas_counters(txn_fee_statements);
                    counters::EXCEED_PER_BLOCK_GAS_LIMIT_COUNT
                        .with_label_values(&[counters::Mode::PARALLEL])
                        .inc();
                    info!("[BlockSTM]: Parallel execution early halted due to accumulated_non_storage_gas {} >= PER_BLOCK_GAS_LIMIT {}, {} txns committed", accumulated_non_storage_gas, per_block_gas_limit, txn_idx);
                    break;
                }
            }

            // Remark: When early halting the BlockSTM, we have to make sure the current / new tasks
            // will be properly handled by the threads. For instance, it is possible that the committing
            // thread holds an execution task of ExecutionTaskType::Wakeup(DependencyCondvar) for some
            // other thread pending on the dependency conditional variable from the last iteration. If
            // the committing thread early halts BlockSTM and resets its scheduler_task to be Done, the
            // pending thread will be pending on read forever. In other words, we rely on the committing
            // thread to wake up the pending execution thread, if the committing thread holds the Wakeup task.
        }
    }

    fn worker_commit_hook(
        &self,
        txn_idx: TxnIndex,
        versioned_cache: &MVHashMap<T::Key, T::Value, X>,
        last_input_output: &TxnLastInputOutput<T::Key, E::Output, E::Error>,
        base_view: &S,
    ) {
        let (num_deltas, delta_keys) = last_input_output.delta_keys(txn_idx);
        let mut delta_writes = Vec::with_capacity(num_deltas);
        for k in delta_keys {
            // Note that delta materialization happens concurrently, but under concurrent
            // commit_hooks (which may be dispatched by the coordinator), threads may end up
            // contending on delta materialization of the same aggregator. However, the
            // materialization is based on previously materialized values and should not
            // introduce long critical sections. Moreover, with more aggregators, and given
            // that the commit_hook will be performed at dispersed times based on the
            // completion of the respective previous tasks of threads, this should not be
            // an immediate bottleneck - confirmed by an experiment with 32 core and a
            // single materialized aggregator. If needed, the contention may be further
            // mitigated by batching consecutive commit_hooks.
            let committed_delta = versioned_cache
                .materialize_delta(&k, txn_idx)
                .unwrap_or_else(|op| {
                    let storage_value = base_view
                        .get_state_value_bytes(&k)
                        .expect("No base value for committed delta in storage")
                        .map(|bytes| deserialize(&bytes))
                        .expect("Cannot deserialize base value for committed delta");

                    versioned_cache.set_aggregator_base_value(&k, storage_value);
                    op.apply_to(storage_value)
                        .expect("Materializing delta w. base value set must succeed")
                });

            // Must contain committed value as we set the base value above.
            delta_writes.push((
                k.clone(),
                WriteOp::Modification(serialize(&committed_delta)),
            ));
        }
        last_input_output.record_delta_writes(txn_idx, delta_writes);
        if let Some(txn_commit_listener) = &self.transaction_commit_hook {
            let txn_output = last_input_output.txn_output(txn_idx).unwrap();
            let execution_status = txn_output.output_status();

            match execution_status {
                ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                    txn_commit_listener.on_transaction_committed(txn_idx, output);
                },
                ExecutionStatus::Abort(_) => {
                    txn_commit_listener.on_execution_aborted(txn_idx);
                },
            }
        }
    }

    fn work_task_with_scope(
        &self,
        executor_arguments: &E::Argument,
        block: &[T],
        last_input_output: &TxnLastInputOutput<T::Key, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Value, X>,
        scheduler: &Scheduler,
        base_view: &S,
        role: CommitRole,
    ) {
        // Make executor for each task. TODO: fast concurrent executor.
        let init_timer = VM_INIT_SECONDS.start_timer();
        let executor = E::init(*executor_arguments);
        drop(init_timer);

        let committing = matches!(role, CommitRole::Coordinator(_));

        let _timer = WORK_WITH_TASK_SECONDS.start_timer();
        let mut scheduler_task = SchedulerTask::NoTask;
        let mut worker_idx = 0;

        let mut accumulated_fee_statement = FeeStatement::zero();
        let mut txn_fee_statements = Vec::with_capacity(block.len());
        loop {
            // Only one thread does try_commit to avoid contention.
            match &role {
                CommitRole::Coordinator(post_commit_txs) => {
                    self.coordinator_commit_hook(
                        self.maybe_block_gas_limit,
                        scheduler,
                        post_commit_txs,
                        &mut worker_idx,
                        &mut scheduler_task,
                        last_input_output,
                        &mut accumulated_fee_statement,
                        &mut txn_fee_statements,
                    );
                },
                CommitRole::Worker(rx) => {
                    while let Ok(txn_idx) = rx.try_recv() {
                        self.worker_commit_hook(
                            txn_idx,
                            versioned_cache,
                            last_input_output,
                            base_view,
                        );
                    }
                },
            }

            scheduler_task = match scheduler_task {
                SchedulerTask::ValidationTask(version_to_validate, wave) => self.validate(
                    version_to_validate,
                    wave,
                    last_input_output,
                    versioned_cache,
                    scheduler,
                ),
                SchedulerTask::ExecutionTask(version_to_execute, ExecutionTaskType::Execution) => {
                    self.execute(
                        version_to_execute,
                        block,
                        last_input_output,
                        versioned_cache,
                        scheduler,
                        &executor,
                        base_view,
                    )
                },
                SchedulerTask::ExecutionTask(_, ExecutionTaskType::Wakeup(condvar)) => {
                    let (lock, cvar) = &*condvar;
                    // Mark dependency resolved.
                    *lock.lock() = DependencyStatus::Resolved;
                    // Wake up the process waiting for dependency.
                    cvar.notify_one();

                    SchedulerTask::NoTask
                },
                SchedulerTask::NoTask => scheduler.next_task(committing),
                SchedulerTask::Done => {
                    // Make sure to drain any remaining commit tasks assigned by the coordinator.
                    if let CommitRole::Worker(rx) = &role {
                        // Until the sender drops the tx, an index for commit_hook might be sent.
                        while let Ok(txn_idx) = rx.recv() {
                            self.worker_commit_hook(
                                txn_idx,
                                versioned_cache,
                                last_input_output,
                                base_view,
                            );
                        }
                    }
                    break;
                },
            }
        }
    }

    pub(crate) fn execute_transactions_parallel(
        &self,
        executor_initial_arguments: E::Argument,
        signature_verified_block: &Vec<T>,
        base_view: &S,
    ) -> Result<Vec<E::Output>, E::Error> {
        let _timer = PARALLEL_EXECUTION_SECONDS.start_timer();
        // Using parallel execution with 1 thread currently will not work as it
        // will only have a coordinator role but no workers for rolling commit.
        // Need to special case no roles (commit hook by thread itself) to run
        // w. concurrency_level = 1 for some reason.
        assert!(self.concurrency_level > 1, "Must use sequential execution");

        let versioned_cache = MVHashMap::new();

        if signature_verified_block.is_empty() {
            return Ok(vec![]);
        }

        let num_txns = signature_verified_block.len() as u32;
        let last_input_output = TxnLastInputOutput::new(num_txns);
        let scheduler = Scheduler::new(num_txns);

        let mut roles: Vec<CommitRole> = vec![];
        let mut senders: Vec<Sender<u32>> = Vec::with_capacity(self.concurrency_level - 1);
        for _ in 0..(self.concurrency_level - 1) {
            let (tx, rx) = mpsc::channel();
            roles.push(CommitRole::Worker(rx));
            senders.push(tx);
        }
        // Add the coordinator role. Coordinator is responsible for committing
        // indices and assigning post-commit work per index to other workers.
        // Note: It is important that the Coordinator is the first thread that
        // picks up a role will be a coordinator. Hence, if multiple parallel
        // executors are running concurrently, they will all have active coordinator.
        roles.push(CommitRole::Coordinator(senders));

        let timer = RAYON_EXECUTION_SECONDS.start_timer();
        self.executor_thread_pool.scope(|s| {
            for _ in 0..self.concurrency_level {
                let role = roles.pop().expect("Role must be set for all threads");
                s.spawn(|_| {
                    self.work_task_with_scope(
                        &executor_initial_arguments,
                        signature_verified_block,
                        &last_input_output,
                        &versioned_cache,
                        &scheduler,
                        base_view,
                        role,
                    );
                });
            }
        });
        drop(timer);

        let num_txns = num_txns as usize;
        // TODO: for large block sizes and many cores, extract outputs in parallel.
        let mut final_results = Vec::with_capacity(num_txns);

        let maybe_err = if last_input_output.module_publishing_may_race() {
            counters::MODULE_PUBLISHING_FALLBACK_COUNT.inc();
            Some(Error::ModulePathReadWrite)
        } else {
            let mut ret = None;
            for idx in 0..num_txns {
                match last_input_output.take_output(idx as TxnIndex) {
                    ExecutionStatus::Success(t) => final_results.push(t),
                    ExecutionStatus::SkipRest(t) => {
                        final_results.push(t);
                        break;
                    },
                    ExecutionStatus::Abort(err) => {
                        ret = Some(err);
                        break;
                    },
                };
            }
            ret
        };

        self.executor_thread_pool.spawn(move || {
            // Explicit async drops.
            drop(last_input_output);
            drop(scheduler);
            // TODO: re-use the code cache.
            drop(versioned_cache);
        });

        match maybe_err {
            Some(err) => Err(err),
            None => {
                final_results.resize_with(num_txns, E::Output::skip_output);
                Ok(final_results)
            },
        }
    }

    pub(crate) fn execute_transactions_sequential(
        &self,
        executor_arguments: E::Argument,
        signature_verified_block: &Vec<T>,
        base_view: &S,
    ) -> Result<Vec<E::Output>, E::Error> {
        let num_txns = signature_verified_block.len();
        let executor = E::init(executor_arguments);
        let data_map = UnsyncMap::new();

        let mut ret = Vec::with_capacity(num_txns);

        let mut accumulated_fee_statement = FeeStatement::zero();

        for (idx, txn) in signature_verified_block.iter().enumerate() {
            let res = executor.execute_transaction(
                &LatestView::<T, S, X>::new_btree_view(base_view, &data_map, idx as TxnIndex),
                txn,
                idx as TxnIndex,
                true,
            );

            let must_skip = matches!(res, ExecutionStatus::SkipRest(_));
            match res {
                ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                    assert_eq!(
                        output.get_deltas().len(),
                        0,
                        "Sequential execution must materialize deltas"
                    );
                    // Apply the writes.
                    for (ap, write_op) in output.get_writes().into_iter() {
                        data_map.write(ap, write_op);
                    }
                    // Calculating the accumulated gas costs of the committed txns.
                    let fee_statement = output.fee_statement();
                    accumulated_fee_statement.add_fee_statement(&fee_statement);
                    Self::update_sequential_txn_gas_counters(&fee_statement);
                    // No delta writes are needed for sequential execution.
                    output.incorporate_delta_writes(vec![]);
                    //
                    if let Some(commit_hook) = &self.transaction_commit_hook {
                        commit_hook.on_transaction_committed(idx as TxnIndex, &output);
                    }
                    ret.push(output);
                },
                ExecutionStatus::Abort(err) => {
                    if let Some(commit_hook) = &self.transaction_commit_hook {
                        commit_hook.on_execution_aborted(idx as TxnIndex);
                    }
                    // Record the status indicating abort.
                    return Err(Error::UserError(err));
                },
            }

            // When the txn is a SkipRest txn, halt sequential execution.
            if must_skip {
                info!("[Execution]: Sequential execution early halted due to SkipRest txn, {} txns committed.", ret.len());
                break;
            }

            if let Some(per_block_gas_limit) = self.maybe_block_gas_limit {
                // When the accumulated gas of the committed txns
                // exceeds per_block_gas_limit, halt sequential execution.
                let accumulated_non_storage_gas = accumulated_fee_statement.execution_gas_used()
                    + accumulated_fee_statement.io_gas_used();
                if accumulated_non_storage_gas >= per_block_gas_limit {
                    counters::EXCEED_PER_BLOCK_GAS_LIMIT_COUNT
                        .with_label_values(&[counters::Mode::SEQUENTIAL])
                        .inc();
                    info!("[Execution]: Sequential execution early halted due to accumulated_non_storage_gas {} >= PER_BLOCK_GAS_LIMIT {}, {} txns committed", accumulated_non_storage_gas, per_block_gas_limit, ret.len());
                    break;
                }
            }
        }

        if ret.len() == num_txns {
            info!(
                "[Execution]: Sequential execution completed, all {} txns committed.",
                ret.len()
            );
        }

        Self::update_sequential_block_gas_counters(&accumulated_fee_statement, ret.len());
        ret.resize_with(num_txns, E::Output::skip_output);
        Ok(ret)
    }

    pub fn execute_block(
        &self,
        executor_arguments: E::Argument,
        signature_verified_block: BlockExecutorTransactions<T>,
        base_view: &S,
    ) -> Result<Vec<E::Output>, E::Error> {
        let signature_verified_txns = signature_verified_block.into_txns();
        let mut ret = if self.concurrency_level > 1 {
            self.execute_transactions_parallel(
                executor_arguments,
                &signature_verified_txns,
                base_view,
            )
        } else {
            self.execute_transactions_sequential(
                executor_arguments,
                &signature_verified_txns,
                base_view,
            )
        };

        if matches!(ret, Err(Error::ModulePathReadWrite)) {
            debug!("[Execution]: Module read & written, sequential fallback");

            // All logs from the parallel execution should be cleared and not reported.
            // Clear by re-initializing the speculative logs.
            init_speculative_logs(signature_verified_txns.len());

            ret = self.execute_transactions_sequential(
                executor_arguments,
                &signature_verified_txns,
                base_view,
            )
        }
        self.executor_thread_pool.spawn(move || {
            // Explicit async drops.
            drop(signature_verified_txns);
        });
        ret
    }
}
