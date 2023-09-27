// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    counters::{
        PARALLEL_EXECUTION_SECONDS, RAYON_EXECUTION_SECONDS, TASK_EXECUTE_SECONDS,
        TASK_VALIDATE_SECONDS, VM_INIT_SECONDS, WORK_WITH_TASK_SECONDS,
    },
    errors::*,
    explicit_sync_wrapper::ExplicitSyncWrapper,
    scheduler::{DependencyStatus, ExecutionTaskType, Scheduler, SchedulerTask, Wave},
    task::{ExecutionStatus, ExecutorTask, Transaction, TransactionOutput},
    txn_commit_hook::TransactionCommitHook,
    txn_last_input_output::TxnLastInputOutput,
    view::{LatestView, ParallelState, SequentialState, ViewState},
};
use aptos_aggregator::delta_change_set::serialize;
use aptos_infallible::Mutex;
use aptos_logger::{debug, info};
use aptos_mvhashmap::{
    types::{Incarnation, TxnIndex},
    unsync_map::UnsyncMap,
    MVHashMap,
};
use aptos_state_view::TStateView;
use aptos_types::{
    executable::Executable,
    fee_statement::FeeStatement,
    write_set::{TransactionWrite, WriteOp},
};
use aptos_vm_logging::{clear_speculative_txn_logs, init_speculative_logs};
use num_cpus;
use rayon::ThreadPool;
use std::{
    collections::HashMap,
    marker::{PhantomData, Sync},
    sync::{atomic::AtomicU32, Arc},
};

pub struct BlockExecutor<T, E, S, L, X> {
    // Number of active concurrent tasks, corresponding to the maximum number of rayon
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

    fn execute(
        idx_to_execute: TxnIndex,
        incarnation: Incarnation,
        signature_verified_block: &[T],
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X>,
        scheduler: &Scheduler,
        executor: &E,
        base_view: &S,
        latest_view: ParallelState<T, X>,
        maybe_error: &Mutex<Option<Error<E::Error>>>,
    ) -> SchedulerTask {
        let _timer = TASK_EXECUTE_SECONDS.start_timer();
        let txn = &signature_verified_block[idx_to_execute as usize];

        // VM execution.
        let sync_view = LatestView::new(base_view, ViewState::Sync(latest_view), idx_to_execute);
        let execute_result = executor.execute_transaction(&sync_view, txn, idx_to_execute, false);

        let mut prev_modified_keys = last_input_output
            .modified_keys(idx_to_execute)
            .map_or(HashMap::new(), |keys| keys.collect());

        // For tracking whether the recent execution wrote outside of the previous write/delta set.
        let mut updates_outside = false;
        let mut apply_updates = |output: &E::Output| {
            // First, apply writes.
            for (k, v) in output
                .resource_write_set()
                .into_iter()
                .chain(output.aggregator_v1_write_set().into_iter())
            {
                if prev_modified_keys.remove(&k).is_none() {
                    updates_outside = true;
                }
                versioned_cache
                    .data()
                    .write(k, idx_to_execute, incarnation, v);
            }

            for (k, v) in output.module_write_set().into_iter() {
                if prev_modified_keys.remove(&k).is_none() {
                    updates_outside = true;
                }
                versioned_cache.modules().write(k, idx_to_execute, v);
            }

            // Then, apply deltas.
            for (k, d) in output.aggregator_v1_delta_set().into_iter() {
                if prev_modified_keys.remove(&k).is_none() {
                    updates_outside = true;
                }
                versioned_cache.data().add_delta(k, idx_to_execute, d);
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
        for (k, is_module) in prev_modified_keys {
            if is_module {
                versioned_cache.modules().delete(&k, idx_to_execute);
            } else {
                versioned_cache.data().delete(&k, idx_to_execute);
            }
        }

        if last_input_output
            .record(idx_to_execute, sync_view.take_reads(), result)
            .is_err()
        {
            let mut maybe_error = maybe_error.lock();
            *maybe_error = Some(Error::ModulePathReadWrite);
            scheduler.halt();
            return SchedulerTask::NoTask;
        }
        scheduler.finish_execution(idx_to_execute, incarnation, updates_outside)
    }

    fn validate(
        idx_to_validate: TxnIndex,
        incarnation: Incarnation,
        validation_wave: Wave,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X>,
        scheduler: &Scheduler,
    ) -> SchedulerTask {
        let _timer = TASK_VALIDATE_SECONDS.start_timer();
        let read_set = last_input_output
            .read_set(idx_to_validate)
            .expect("[BlockSTM]: Prior read-set must be recorded");

        // TODO: validate modules when there is no r/w fallback.
        let valid = read_set.validate_data_reads(versioned_cache.data(), idx_to_validate)
            && read_set.validate_group_reads(versioned_cache.group_data(), idx_to_validate);

        let aborted = !valid && scheduler.try_abort(idx_to_validate, incarnation);

        if aborted {
            counters::SPECULATIVE_ABORT_COUNT.inc();

            // Any logs from the aborted execution should be cleared and not reported.
            clear_speculative_txn_logs(idx_to_validate as usize);

            // Not valid and successfully aborted, mark the latest write/delta sets as estimates.
            if let Some(keys) = last_input_output.modified_keys(idx_to_validate) {
                for (k, is_module_path) in keys {
                    if is_module_path {
                        versioned_cache.modules().mark_estimate(&k, idx_to_validate);
                    } else {
                        versioned_cache.data().mark_estimate(&k, idx_to_validate);
                    }
                }
            }

            scheduler.finish_abort(idx_to_validate, incarnation)
        } else {
            scheduler.finish_validation(idx_to_validate, validation_wave);

            if valid {
                scheduler.queueing_commits_arm();
            }

            SchedulerTask::NoTask
        }
    }

    fn prepare_and_queue_commit_ready_txns(
        &self,
        maybe_block_gas_limit: Option<u64>,
        scheduler: &Scheduler,
        scheduler_task: &mut SchedulerTask,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        shared_commit_state: &ExplicitSyncWrapper<(
            FeeStatement,
            Vec<FeeStatement>,
            Mutex<Option<Error<E::Error>>>,
        )>,
    ) {
        while let Some(txn_idx) = scheduler.try_commit() {
            let (accumulated_fee_statement, txn_fee_statements, maybe_error) =
                shared_commit_state.get_mut();

            defer! {
                scheduler.add_to_commit_queue(txn_idx);
            }

            if let Some(fee_statement) = last_input_output.fee_statement(txn_idx) {
                // For committed txns with Success status, calculate the accumulated gas costs.
                accumulated_fee_statement.add_fee_statement(&fee_statement);
                txn_fee_statements.push(fee_statement);

                if let Some(per_block_gas_limit) = maybe_block_gas_limit {
                    // When the accumulated execution and io gas of the committed txns exceeds
                    // PER_BLOCK_GAS_LIMIT, early halt BlockSTM. Storage gas does not count towards
                    // the per block gas limit, as we measure execution related cost here.
                    let accumulated_non_storage_gas = accumulated_fee_statement
                        .execution_gas_used()
                        + accumulated_fee_statement.io_gas_used();
                    if accumulated_non_storage_gas >= per_block_gas_limit {
                        counters::EXCEED_PER_BLOCK_GAS_LIMIT_COUNT
                            .with_label_values(&[counters::Mode::PARALLEL])
                            .inc();
                        info!(
                            "[BlockSTM]: Parallel execution early halted due to \
                             accumulated_non_storage_gas {} >= PER_BLOCK_GAS_LIMIT {}",
                            accumulated_non_storage_gas, per_block_gas_limit,
                        );

                        // Set the execution output status to be SkipRest, to skip the rest of the txns.
                        last_input_output.update_to_skip_rest(txn_idx);
                    }
                }
            }

            if last_input_output.execution_error(txn_idx).is_some() {
                let mut maybe_error = maybe_error.lock();
                *maybe_error = last_input_output.execution_error(txn_idx);
                info!(
                    "Block execution was aborted due to {:?}",
                    maybe_error.as_ref().unwrap()
                );
                scheduler.halt();
                break;
            }

            // Committed the last transaction, BlockSTM finishes execution.
            if txn_idx + 1 == scheduler.num_txns()
                || last_input_output.block_truncated_at_idx(txn_idx)
            {
                if txn_idx + 1 == scheduler.num_txns() {
                    assert!(
                        !matches!(scheduler_task, SchedulerTask::ExecutionTask(_, _, _)),
                        "All transactions can be committed, can't have execution task"
                    );
                }

                // Either all txn committed, or a committed txn caused an early halt.
                scheduler.halt();

                counters::update_parallel_block_gas_counters(
                    accumulated_fee_statement,
                    (txn_idx + 1) as usize,
                );
                counters::update_parallel_txn_gas_counters(txn_fee_statements);

                let accumulated_non_storage_gas = accumulated_fee_statement.execution_gas_used()
                    + accumulated_fee_statement.io_gas_used();
                info!(
                    "[BlockSTM]: Parallel execution completed. {} out of {} txns committed. \
		     accumulated_non_storage_gas = {}, limit = {:?}",
                    txn_idx + 1,
                    scheduler.num_txns(),
                    accumulated_non_storage_gas,
                    maybe_block_gas_limit,
                );
                break;
            }
        }
    }

    fn materialize_txn_commit(
        &self,
        txn_idx: TxnIndex,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X>,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        base_view: &S,
        final_results: &ExplicitSyncWrapper<Vec<E::Output>>,
    ) {
        let delta_keys = last_input_output.delta_keys(txn_idx);
        let _events = last_input_output.events(txn_idx);
        let mut delta_writes = Vec::with_capacity(delta_keys.len());
        for k in delta_keys.into_iter() {
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
                .data()
                .materialize_delta(&k, txn_idx)
                .unwrap_or_else(|op| {
                    // TODO: this logic should improve with the new AGGR data structure
                    // TODO: and the ugly base_view parameter will also disappear.
                    let storage_value = base_view
                        .get_state_value(&k)
                        .expect("Error reading the base value for committed delta in storage");

                    let w: T::Value = TransactionWrite::from_state_value(storage_value);
                    let value_u128 = w
                        .as_u128()
                        .expect("Aggregator base value deserialization error")
                        .expect("Aggregator base value must exist");

                    versioned_cache.data().provide_base_value(k.clone(), w);
                    op.apply_to(value_u128)
                        .expect("Materializing delta w. base value set must succeed")
                });

            // Must contain committed value as we set the base value above.
            delta_writes.push((k, WriteOp::Modification(serialize(&committed_delta).into())));
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

        let final_results = final_results.get_mut();
        match last_input_output.take_output(txn_idx) {
            ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                final_results[txn_idx as usize] = t;
            },
            ExecutionStatus::Abort(_) => (),
        };
    }

    fn worker_loop(
        &self,
        executor_arguments: &E::Argument,
        block: &[T],
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X>,
        scheduler: &Scheduler,
        // TODO: should not need to pass base view.
        base_view: &S,
        shared_counter: &AtomicU32,
        shared_commit_state: &ExplicitSyncWrapper<(
            FeeStatement,
            Vec<FeeStatement>,
            Mutex<Option<Error<E::Error>>>,
        )>,
        final_results: &ExplicitSyncWrapper<Vec<E::Output>>,
    ) {
        // Make executor for each task. TODO: fast concurrent executor.
        let init_timer = VM_INIT_SECONDS.start_timer();
        let executor = E::init(*executor_arguments);
        drop(init_timer);

        let _timer = WORK_WITH_TASK_SECONDS.start_timer();
        let mut scheduler_task = SchedulerTask::NoTask;

        loop {
            // Priorotize committing validated transactions
            while scheduler.should_coordinate_commits() {
                self.prepare_and_queue_commit_ready_txns(
                    self.maybe_block_gas_limit,
                    scheduler,
                    &mut scheduler_task,
                    last_input_output,
                    shared_commit_state,
                );
                scheduler.queueing_commits_mark_done();
            }

            {
                while let Ok(txn_idx) = scheduler.pop_from_commit_queue() {
                    self.materialize_txn_commit(
                        txn_idx,
                        versioned_cache,
                        last_input_output,
                        base_view,
                        final_results,
                    );
                }
            }

            scheduler_task = match scheduler_task {
                SchedulerTask::ValidationTask(txn_idx, incarnation, wave) => Self::validate(
                    txn_idx,
                    incarnation,
                    wave,
                    last_input_output,
                    versioned_cache,
                    scheduler,
                ),
                SchedulerTask::ExecutionTask(
                    txn_idx,
                    incarnation,
                    ExecutionTaskType::Execution,
                ) => {
                    let (_, _, maybe_error) = shared_commit_state.get();
                    Self::execute(
                        txn_idx,
                        incarnation,
                        block,
                        last_input_output,
                        versioned_cache,
                        scheduler,
                        &executor,
                        base_view,
                        ParallelState::new(versioned_cache, scheduler, shared_counter),
                        maybe_error,
                    )
                },
                SchedulerTask::ExecutionTask(_, _, ExecutionTaskType::Wakeup(condvar)) => {
                    let (lock, cvar) = &*condvar;
                    // Mark dependency resolved.
                    *lock.lock() = DependencyStatus::Resolved;
                    // Wake up the process waiting for dependency.
                    cvar.notify_one();

                    SchedulerTask::NoTask
                },
                SchedulerTask::NoTask => scheduler.next_task(),
                SchedulerTask::Done => {
                    while let Ok(txn_idx) = scheduler.pop_from_commit_queue() {
                        self.materialize_txn_commit(
                            txn_idx,
                            versioned_cache,
                            last_input_output,
                            base_view,
                            final_results,
                        );
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
        let shared_counter = AtomicU32::new(0);

        if signature_verified_block.is_empty() {
            return Ok(vec![]);
        }

        let num_txns = signature_verified_block.len();

        let shared_commit_state = ExplicitSyncWrapper::new((
            FeeStatement::zero(),
            Vec::<FeeStatement>::with_capacity(num_txns),
            Mutex::new(None),
        ));

        let final_results = ExplicitSyncWrapper::new(Vec::with_capacity(num_txns));

        {
            final_results
                .get_mut()
                .resize_with(num_txns, E::Output::skip_output);
        }

        let num_txns = num_txns as u32;

        let last_input_output = TxnLastInputOutput::new(num_txns);
        let scheduler = Scheduler::new(num_txns);

        let timer = RAYON_EXECUTION_SECONDS.start_timer();
        self.executor_thread_pool.scope(|s| {
            for _ in 0..self.concurrency_level {
                s.spawn(|_| {
                    self.worker_loop(
                        &executor_initial_arguments,
                        signature_verified_block,
                        &last_input_output,
                        &versioned_cache,
                        &scheduler,
                        base_view,
                        &shared_counter,
                        &shared_commit_state,
                        &final_results,
                    );
                });
            }
        });
        drop(timer);

        self.executor_thread_pool.spawn(move || {
            // Explicit async drops.
            drop(last_input_output);
            drop(scheduler);
            // TODO: re-use the code cache.
            drop(versioned_cache);
        });

        let (_, _, maybe_error) = shared_commit_state.into_inner();
        match maybe_error.into_inner() {
            Some(err) => Err(err),
            None => Ok(final_results.into_inner()),
        }
    }

    pub(crate) fn execute_transactions_sequential(
        &self,
        executor_arguments: E::Argument,
        signature_verified_block: &Vec<T>,
        base_view: &S,
    ) -> Result<Vec<E::Output>, E::Error> {
        let num_txns = signature_verified_block.len();
        let init_timer = VM_INIT_SECONDS.start_timer();
        let executor = E::init(executor_arguments);
        drop(init_timer);

        let data_map = UnsyncMap::new();
        let mut ret = Vec::with_capacity(num_txns);
        let mut accumulated_fee_statement = FeeStatement::zero();

        for (idx, txn) in signature_verified_block.iter().enumerate() {
            let unsync_view = LatestView::<T, S, X>::new(
                base_view,
                ViewState::Unsync(SequentialState {
                    unsync_map: &data_map,
                    _counter: &0,
                }),
                idx as TxnIndex,
            );
            let res = executor.execute_transaction(&unsync_view, txn, idx as TxnIndex, true);

            let must_skip = matches!(res, ExecutionStatus::SkipRest(_));
            match res {
                ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                    assert_eq!(
                        output.aggregator_v1_delta_set().len(),
                        0,
                        "Sequential execution must materialize deltas"
                    );
                    // Apply the writes.
                    for (key, write_op) in output
                        .resource_write_set()
                        .into_iter()
                        .chain(output.aggregator_v1_write_set().into_iter())
                        .chain(output.module_write_set().into_iter())
                    {
                        data_map.write(key, write_op);
                    }
                    // Calculating the accumulated gas costs of the committed txns.
                    let fee_statement = output.fee_statement();
                    accumulated_fee_statement.add_fee_statement(&fee_statement);
                    counters::update_sequential_txn_gas_counters(&fee_statement);

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
                    info!(
                        "[Execution]: Sequential execution early halted due to \
                        accumulated_non_storage_gas {} >= PER_BLOCK_GAS_LIMIT {}, {} txns committed.",
                        accumulated_non_storage_gas,
                        per_block_gas_limit,
                        ret.len()
                    );
                    break;
                }
            }
        }

        if ret.len() == num_txns {
            let accumulated_non_storage_gas = accumulated_fee_statement.execution_gas_used()
                + accumulated_fee_statement.io_gas_used();
            info!(
                "[Execution]: Sequential execution completed. \
		 {} out of {} txns committed. accumulated_non_storage_gas = {}, limit = {:?}",
                ret.len(),
                num_txns,
                accumulated_non_storage_gas,
                self.maybe_block_gas_limit,
            );
        }

        counters::update_sequential_block_gas_counters(&accumulated_fee_statement, ret.len());
        ret.resize_with(num_txns, E::Output::skip_output);
        Ok(ret)
    }

    pub fn execute_block(
        &self,
        executor_arguments: E::Argument,
        signature_verified_block: Vec<T>,
        base_view: &S,
    ) -> Result<Vec<E::Output>, E::Error> {
        let mut ret = if self.concurrency_level > 1 {
            self.execute_transactions_parallel(
                executor_arguments,
                &signature_verified_block,
                base_view,
            )
        } else {
            self.execute_transactions_sequential(
                executor_arguments,
                &signature_verified_block,
                base_view,
            )
        };

        if matches!(ret, Err(Error::ModulePathReadWrite)) {
            debug!("[Execution]: Module read & written, sequential fallback");

            // All logs from the parallel execution should be cleared and not reported.
            // Clear by re-initializing the speculative logs.
            init_speculative_logs(signature_verified_block.len());

            ret = self.execute_transactions_sequential(
                executor_arguments,
                &signature_verified_block,
                base_view,
            )
        }
        self.executor_thread_pool.spawn(move || {
            // Explicit async drops.
            drop(signature_verified_block);
        });
        ret
    }
}
