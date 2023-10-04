// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    captured_reads::DataRead,
    counters,
    counters::{
        PARALLEL_EXECUTION_SECONDS, RAYON_EXECUTION_SECONDS, TASK_EXECUTE_SECONDS,
        TASK_VALIDATE_SECONDS, VM_INIT_SECONDS, WORK_WITH_TASK_SECONDS,
    },
    errors::*,
    scheduler::{DependencyStatus, ExecutionTaskType, Scheduler, SchedulerTask, Wave},
    task::{
        CategorizeError, ErrorCategory, ExecutionStatus, ExecutorTask, Transaction,
        TransactionOutput,
    },
    txn_commit_hook::TransactionCommitHook,
    txn_last_input_output::TxnLastInputOutput,
    view::{LatestView, ParallelState, SequentialState, ViewState},
};
use aptos_aggregator::{
    delayed_change::{ApplyBase, DelayedChange},
    delta_change_set::serialize,
    types::{expect_ok, PanicOr},
};
use aptos_logger::{debug, info};
use aptos_mvhashmap::{
    types::{Incarnation, MVDelayedFieldsError, TxnIndex},
    unsync_map::UnsyncMap,
    versioned_delayed_fields::CommitError,
    MVHashMap,
};
use aptos_state_view::TStateView;
use aptos_types::{
    contract_event::ReadWriteEvent,
    executable::Executable,
    fee_statement::FeeStatement,
    write_set::{TransactionWrite, WriteOp},
};
use aptos_vm_logging::{clear_speculative_txn_logs, init_speculative_logs};
use bytes::Bytes;
use claims::assert_none;
use num_cpus;
use rayon::ThreadPool;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    sync::{
        atomic::AtomicU32,
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
    E::Error: CategorizeError,
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
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
        scheduler: &Scheduler,
        executor: &E,
        base_view: &S,
        latest_view: ParallelState<T, X>,
    ) -> SchedulerTask {
        let _timer = TASK_EXECUTE_SECONDS.start_timer();
        let txn = &signature_verified_block[idx_to_execute as usize];

        // VM execution.
        let sync_view = LatestView::new(base_view, ViewState::Sync(latest_view), idx_to_execute);
        let execute_result = executor.execute_transaction(&sync_view, txn, idx_to_execute, false);

        let mut prev_modified_keys = last_input_output
            .modified_keys(idx_to_execute)
            .map_or(HashMap::new(), |keys| keys.collect());

        // let mut prev_modified_aggregators = last_input_output
        //     .delayed_field_keys(idx_to_execute)
        //     .map_or(HashSet::new(), |keys| keys.collect());

        let mut speculative_inconsistent = false;

        // For tracking whether the recent execution wrote outside of the previous write/delta set.
        let mut updates_outside = false;
        let mut apply_updates = |output: &E::Output| {
            // First, apply writes.
            for (k, v) in output.resource_write_set().into_iter().chain(
                output
                    .aggregator_v1_write_set()
                    .into_iter()
                    .map(|(state_key, write_op)| (state_key, (write_op, None))),
            ) {
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

            for (id, change) in output.delayed_field_change_set().into_iter() {
                // if !prev_modified_aggregators.remove(&id) {
                //     updates_outside = true;
                // }

                // TODO: figure out if change should update updates_outside
                if let Err(e) =
                    versioned_cache
                        .delayed_fields()
                        .record_change(id, idx_to_execute, change)
                {
                    match e {
                        PanicOr::CodeInvariantError(m) => panic!("{}", m),
                        PanicOr::Or(_) => speculative_inconsistent = true,
                    };
                }
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
                match err.categorize() {
                    ErrorCategory::CodeInvariantError => {
                        // TODO fallback to speculative execution
                        panic!("");
                    },
                    ErrorCategory::SpeculativeExecutionError => {
                        speculative_inconsistent = true;
                    },
                    _ => (),
                };

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

        let mut read_set = sync_view.take_reads();
        if speculative_inconsistent {
            read_set.capture_delayed_field_read_error(&PanicOr::Or(
                MVDelayedFieldsError::DeltaApplicationFailure,
            ));
        }

        if last_input_output
            .record(idx_to_execute, read_set, result)
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
        idx_to_validate: TxnIndex,
        incarnation: Incarnation,
        validation_wave: Wave,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
        scheduler: &Scheduler,
    ) -> SchedulerTask {
        let _timer = TASK_VALIDATE_SECONDS.start_timer();
        let read_set = last_input_output
            .read_set(idx_to_validate)
            .expect("[BlockSTM]: Prior read-set must be recorded");

        if read_set.validate_incorrect_use() {
            // TODO fallback to speculative
            panic!("Incorrect use !");
        }

        // Note: we validate delayed field reads only at try_commit.
        // TODO: potentially add some basic validation.
        // TODO: potentially add more sophisticated validation, but if it fails,
        // we mark it as a soft failure, requires some new statuses in the scheduler
        // (i.e. not re-execute unless some other part of the validation fails or
        // until commit, but mark as estimates).

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

            if let Some(keys) = last_input_output.delayed_field_keys(idx_to_validate) {
                for k in keys {
                    versioned_cache
                        .delayed_fields()
                        .mark_estimate(&k, idx_to_validate);
                }
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
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
        post_commit_txs: &Vec<Sender<u32>>,
        worker_idx: &mut usize,
        scheduler_task: &mut SchedulerTask,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        accumulated_fee_statement: &mut FeeStatement,
        txn_fee_statements: &mut Vec<FeeStatement>,
    ) {
        while let Some(txn_idx) = scheduler.try_commit() {
            let read_set = last_input_output
                .read_set(txn_idx)
                .expect("Read set must be recorded");
            let mut execution_still_valid =
                read_set.validate_delayed_field_reads(versioned_cache.delayed_fields(), txn_idx);

            match last_input_output.output_category(txn_idx) {
                Some(ErrorCategory::SpeculativeExecutionError) => {
                    assert!(!execution_still_valid);
                },
                Some(ErrorCategory::CodeInvariantError) => {
                    panic!();
                },
                _ => (),
            };

            if execution_still_valid {
                if let Some(delayed_field_ids) = last_input_output.delayed_field_keys(txn_idx) {
                    if let Err(e) = versioned_cache
                        .delayed_fields()
                        .try_commit(txn_idx, delayed_field_ids.collect())
                    {
                        match e {
                            CommitError::ReExecutionNeeded(_) => {
                                execution_still_valid = false;
                            },
                            CommitError::CodeInvariantError(_) => {
                                // TODO: fallback to sequential execution
                                panic!();
                            },
                        }
                    }
                }
            }
            if !execution_still_valid {
                // TODO call to re-execute transaction
                panic!();
            }

            // Create a CommitGuard to ensure Coordinator sends the committed txn index to Worker.
            let _commit_guard: CommitGuard =
                CommitGuard::new(post_commit_txs, *worker_idx, txn_idx);
            // Iterate round robin over workers to do commit_hook.
            *worker_idx = (*worker_idx + 1) % post_commit_txs.len();

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

            // Committed the last transaction, BlockSTM finishes execution.
            if txn_idx + 1 == scheduler.num_txns()
                || last_input_output.block_truncated_at_idx(txn_idx)
            {
                if txn_idx + 1 == scheduler.num_txns() {
                    assert!(
                        !matches!(scheduler_task, SchedulerTask::ExecutionTask(_, _, _)),
                        "All transactions can be committed, can't have execution task"
                    );

                    // The caller should finish the worker loop.
                    *scheduler_task = SchedulerTask::Done;
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
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
        scheduler: &Scheduler,
        shared_counter: &AtomicU32,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        base_view: &S,
    ) {
        println!("worker commit hook called");
        let parallel_state = ParallelState::<T, X>::new(versioned_cache, scheduler, shared_counter);
        let latest_view = LatestView::new(base_view, ViewState::Sync(parallel_state), txn_idx);

        // For each delayed field in resource write set, replace the identifiers with values.
        let mut write_set_keys = HashSet::new();
        let resource_write_set = last_input_output.resource_write_set(txn_idx);
        let mut patched_resource_write_set = HashMap::new();
        if let Some(resource_write_set) = resource_write_set {
            for (key, (write_op, layout)) in resource_write_set.iter() {
                // layout is Some(_) if it contains a delayed field
                if let Some(layout) = layout {
                    if !write_op.is_deletion() {
                        write_set_keys.insert(key.clone());
                        let patched_bytes = match latest_view
                            .replace_identifiers_with_values(write_op.bytes().unwrap(), layout)
                        {
                            Ok((bytes, _)) => bytes,
                            Err(_) => unreachable!("Failed to replace identifiers with values"),
                        };
                        let mut patched_write_op = write_op.clone();
                        patched_write_op.set_bytes(patched_bytes);
                        patched_resource_write_set.insert(key.clone(), patched_write_op);
                    }
                }
            }
        }

        // For each resource that satisfies the following conditions,
        //     1. Resource is in read set
        //     2. Resource is not in write set
        // replace the delayed field identifiers in the resource with corresponding values.
        // If any of the delayed field identifiers in the resource are part of delayed_field_write_set,
        // then include the resource in the write set.
        let delayed_field_keys = last_input_output.delayed_field_keys(txn_idx);
        if let Some(delayed_field_keys) = delayed_field_keys {
            let delayed_field_keys = delayed_field_keys.collect::<HashSet<_>>();
            let read_set = last_input_output.read_set(txn_idx);
            if let Some(read_set) = read_set {
                for (key, data_read) in read_set.as_ref().data_reads.iter() {
                    if write_set_keys.contains(key) {
                        continue;
                    }
                    // layout is Some(_) if it contains an delayed field
                    if let DataRead::Versioned(_version, value, Some(layout)) = data_read {
                        if let Some(value_bytes) = value.bytes() {
                            match latest_view.replace_identifiers_with_values(value_bytes, layout) {
                                Ok((patched_bytes, delayed_field_keys_in_resource)) => {
                                    if !delayed_field_keys
                                        .is_disjoint(&delayed_field_keys_in_resource)
                                    {
                                        let mut patched_value = value.as_ref().clone();
                                        patched_value.set_bytes(patched_bytes);
                                        patched_resource_write_set
                                            .insert(key.clone(), patched_value);
                                    }
                                },
                                Err(_) => unreachable!(
                                    "Failed to replace identifiers with values in read set"
                                ),
                            };
                        } else {
                            // TODO: Is this unreachable?
                            unreachable!("Data read value must exist");
                        }
                    }
                }
            }
        }

        // For each delayed field in the event, replace delayed field identifier with value.
        let events = last_input_output.events(txn_idx);
        let mut patched_events = vec![];
        for (event, layout) in events {
            if let Some(layout) = layout {
                let (_, _, _, event_data) = event.get_event_data();
                match latest_view
                    .replace_identifiers_with_values(&Bytes::from(event_data.to_vec()), &layout)
                {
                    Ok((bytes, _)) => {
                        let mut patched_event = event.clone();
                        patched_event.update_event_data(bytes.to_vec());
                        patched_events.push(patched_event);
                    },
                    Err(_) => unreachable!("Failed to replace identifiers with values in event"),
                }
            } else {
                patched_events.push(event);
            }
        }

        // Materialize all the aggregator v1 deltas.
        let aggregator_v1_delta_keys = last_input_output.aggregator_v1_delta_keys(txn_idx);
        let mut aggregator_v1_delta_writes = Vec::with_capacity(aggregator_v1_delta_keys.len());
        for k in aggregator_v1_delta_keys.into_iter() {
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
            aggregator_v1_delta_writes
                .push((k, WriteOp::Modification(serialize(&committed_delta).into())));
        }

        last_input_output.record_materialized_txn_output(
            txn_idx,
            aggregator_v1_delta_writes,
            patched_resource_write_set,
            patched_events,
        );
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
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
        scheduler: &Scheduler,
        // TODO: should not need to pass base view.
        base_view: &S,
        shared_counter: &AtomicU32,
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
                        versioned_cache,
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
                            scheduler,
                            shared_counter,
                            last_input_output,
                            base_view,
                        );
                    }
                },
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
                ) => Self::execute(
                    txn_idx,
                    incarnation,
                    block,
                    last_input_output,
                    versioned_cache,
                    scheduler,
                    &executor,
                    base_view,
                    ParallelState::new(versioned_cache, scheduler, shared_counter),
                ),
                SchedulerTask::ExecutionTask(_, _, ExecutionTaskType::Wakeup(condvar)) => {
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
                                scheduler,
                                shared_counter,
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
        let shared_counter = AtomicU32::new(0);

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
                        &shared_counter,
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
        let init_timer = VM_INIT_SECONDS.start_timer();
        let executor = E::init(executor_arguments);
        drop(init_timer);

        let counter = RefCell::new(0);
        let data_map = UnsyncMap::new();
        let mut ret = Vec::with_capacity(num_txns);

        let mut accumulated_fee_statement = FeeStatement::zero();

        for (idx, txn) in signature_verified_block.iter().enumerate() {
            let unsync_view = LatestView::<T, S, X>::new(
                base_view,
                ViewState::Unsync(SequentialState {
                    unsync_map: &data_map,
                    counter: &counter,
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
                        .map(|(k, (v, _))| (k, v))
                        .chain(output.aggregator_v1_write_set().into_iter())
                        .chain(output.module_write_set().into_iter())
                    {
                        data_map.write(key, write_op);
                    }

                    // Calculating the accumulated gas costs of the committed txns.
                    let fee_statement = output.fee_statement();
                    accumulated_fee_statement.add_fee_statement(&fee_statement);
                    counters::update_sequential_txn_gas_counters(&fee_statement);

                    // TODO for materialization, we need to understand what we read,
                    // or do exchange on every transaction, so this logic might change
                    let mut second_phase = Vec::new();
                    let mut updates = HashMap::new();
                    for (id, change) in output.delayed_field_change_set().into_iter() {
                        match change {
                            DelayedChange::Create(value) => {
                                assert_none!(
                                    data_map.fetch_delayed_field(&id),
                                    "Sequential execution must not create duplicate aggregators"
                                );
                                updates.insert(id, value);
                            },
                            DelayedChange::Apply(apply) => {
                                match apply.get_apply_base_id(&id) {
                                    ApplyBase::Previous(base_id) => {
                                        updates.insert(
                                            id,
                                            expect_ok(apply.apply_to_base(
                                                data_map.fetch_delayed_field(&base_id).unwrap(),
                                            ))
                                            .unwrap(),
                                        );
                                    },
                                    ApplyBase::Current(base_id) => {
                                        second_phase.push((id, base_id, apply));
                                    },
                                };
                            },
                        }
                    }
                    for (id, base_id, apply) in second_phase.into_iter() {
                        updates.insert(
                            id,
                            expect_ok(apply.apply_to_base(
                                updates.get(&base_id).cloned().unwrap_or_else(|| {
                                    data_map.fetch_delayed_field(&base_id).unwrap()
                                }),
                            ))
                            .unwrap(),
                        );
                    }
                    for (id, value) in updates.into_iter() {
                        data_map.write_delayed_field(id, value);
                    }

                    // No delta writes are needed for sequential execution.
                    // TODO: Should we replace this with output.incorporate_materialized_txn_output(..)?
                    output.incorporate_delta_writes(vec![]);

                    //
                    if let Some(commit_hook) = &self.transaction_commit_hook {
                        commit_hook.on_transaction_committed(idx as TxnIndex, &output);
                    }
                    ret.push(output);
                },
                ExecutionStatus::Abort(err) => {
                    match err.categorize() {
                        ErrorCategory::CodeInvariantError
                        | ErrorCategory::SpeculativeExecutionError => panic!(
                            "Sequential execution must not have delayed fields errors: {:?}",
                            err
                        ),
                        _ => (),
                    };

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
