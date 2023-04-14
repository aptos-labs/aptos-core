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
    scheduler::{Scheduler, SchedulerTask, Wave},
    task::{ExecutionStatus, ExecutorTask, Transaction, TransactionOutput},
    txn_last_input_output::{ReadDescriptor, TxnLastInputOutput},
    view::{LatestView, MVHashMapView},
};
use aptos_aggregator::delta_change_set::{deserialize, serialize};
use aptos_executable_store::ExecutableStore;
use aptos_logger::{debug, info};
use aptos_mvhashmap::{
    types::{MVCodeError, MVCodeOutput, MVDataError, MVDataOutput, TxnIndex, Version},
    unsync_map::UnsyncMap,
    MVHashMap,
};
use aptos_state_view::TStateView;
use aptos_types::{executable::Executable, write_set::WriteOp};
use aptos_vm_logging::{clear_speculative_txn_logs, init_speculative_logs};
use num_cpus;
use once_cell::sync::Lazy;
use std::{
    marker::PhantomData,
    sync::{
        mpsc,
        mpsc::{Receiver, Sender},
        Arc,
    },
};

#[derive(Debug)]
enum CommitRole {
    Coordinator(Vec<Sender<TxnIndex>>, usize),
    Worker(Receiver<TxnIndex>),
}

pub static RAYON_EXEC_POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .thread_name(|index| format!("par_exec_{}", index))
        .build()
        .unwrap()
});

pub struct BlockExecutor<T, E, S, X> {
    // number of active concurrent tasks, corresponding to the maximum number of rayon
    // threads that may be concurrently participating in parallel execution.
    concurrency_level: usize,
    phantom: PhantomData<(T, E, S, X)>,
}

impl<T, E, S, X> BlockExecutor<T, E, S, X>
where
    T: Transaction,
    E: ExecutorTask<Txn = T>,
    S: TStateView<Key = T::Key> + Sync,
    X: Executable + Send + Sync + 'static,
{
    /// The caller needs to ensure that concurrency_level > 1 (0 is illegal and 1 should
    /// be handled by sequential execution) and that concurrency_level <= num_cpus.
    pub fn new(concurrency_level: usize) -> Self {
        assert!(
            concurrency_level > 0 && concurrency_level <= num_cpus::get(),
            "Parallel execution concurrency level {} should be between 1 and number of CPUs",
            concurrency_level
        );
        Self {
            concurrency_level,
            phantom: PhantomData,
        }
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
                versioned_cache.write(&k, write_version, v);
            }

            // Then, apply deltas.
            for (k, d) in output.get_deltas().into_iter() {
                if !prev_modified_keys.remove(&k) {
                    updates_outside = true;
                }
                versioned_cache.add_delta(&k, idx_to_execute, d);
            }
            // TODO: get module writes separately (required anyway when we have different types
            // for intermediate representations, e.g. MoveValues), and write to the MV data
            // structure without any dynamic dispatching on access_path (expensive as well).
            // We can also assert that modules can't be deleted (here or earlier).
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

        let (captured_reads, captured_executables) = speculative_view.take_captured_inputs();
        last_input_output.record(idx_to_execute, captured_reads, captured_executables, result);
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
        let inputs = last_input_output.inputs(idx_to_validate);

        let mut valid = inputs.reads().iter().all(|(k, r)| {
            match versioned_cache.fetch_data(k, idx_to_validate) {
                Ok(Versioned(version, _)) => *r == ReadDescriptor::Version(version.0, version.1),
                Ok(Resolved(value)) => *r == ReadDescriptor::Resolved(value),
                // Dependency implies a validation failure, and if the original read were to
                // observe an unresolved delta, it would set the aggregator base value in the
                // multi-versioned data-structure, resolve, and record the resolved value.
                Err(Dependency(_)) | Err(Unresolved(_)) => false,
                Err(NotFound) => *r == ReadDescriptor::Storage,
                // We successfully validate when read (again) results in a delta application
                // failure. If the failure is speculative, a later validation will fail due to
                // a read without this error. However, if the failure is real, passing
                // validation here allows to avoid infinitely looping and instead panic when
                // materializing deltas as writes in the final output preparation state. Panic
                // is also preferrable as it allows testing for this scenario.
                Err(DeltaApplicationFailure) => *r == ReadDescriptor::DeltaApplicationFailure,
            }
        });

        valid = valid
            && inputs.executables().iter().all(|(k, r)| {
                // TODO: don't fail code validation on dependency, compare hash after
                // the dependency has finished execution.
                match versioned_cache.fetch_code(k, idx_to_validate) {
                    Ok(MVCodeOutput::Executable((_, desc))) => *r == desc,
                    Ok(MVCodeOutput::Module(_))
                    | Err(MVCodeError::NotFound)
                    | Err(MVCodeError::Dependency(_)) => false,
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

    fn commit_hook(
        &self,
        txn_idx: TxnIndex,
        versioned_cache: &MVHashMap<T::Key, T::Value, X>,
        last_input_output: &TxnLastInputOutput<T::Key, E::Output, E::Error>,
        base_view: &S,
    ) {
        let (num_deltas, delta_keys) = last_input_output.delta_keys(txn_idx);
        let mut delta_writes = Vec::with_capacity(num_deltas);
        for k in delta_keys {
            // Note that delta materialization happens concurrenty, but under concurrent
            // commit_hooks (which may be dispatched by the coordinator), threads may end up
            // contending on delta materialization of the same aggregator. However, the
            // materialization is based on previously materialized values and should not
            // introduce long critical sections. Moreover, with more aggregators, and given
            // that the commit_hook will be performed at dispersed times based on the
            // completion of the respetive previous tasks of threads, this should not be
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

        let committing = matches!(role, CommitRole::Coordinator(_, _));

        let _timer = WORK_WITH_TASK_SECONDS.start_timer();
        let mut scheduler_task = SchedulerTask::NoTask;
        loop {
            // Only one thread does try_commit to avoid contention.
            match &role {
                CommitRole::Coordinator(post_commit_txs, mut idx) => {
                    while let Some(txn_idx) = scheduler.try_commit() {
                        post_commit_txs[idx]
                            .send(txn_idx)
                            .expect("Worker must be available");
                        // Iterate round robin over workers to do commit_hook.
                        idx = (idx + 1) % post_commit_txs.len();

                        if txn_idx as usize + 1 == block.len() {
                            // Committed the last transaction / everything.
                            scheduler_task = SchedulerTask::Done;
                            break;
                        }
                    }
                },
                CommitRole::Worker(rx) => {
                    while let Ok(txn_idx) = rx.try_recv() {
                        self.commit_hook(txn_idx, versioned_cache, last_input_output, base_view);
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
                SchedulerTask::ExecutionTask(version_to_execute, None) => self.execute(
                    version_to_execute,
                    block,
                    last_input_output,
                    versioned_cache,
                    scheduler,
                    &executor,
                    base_view,
                ),
                SchedulerTask::ExecutionTask(_, Some(condvar)) => {
                    let (lock, cvar) = &*condvar;
                    // Mark dependency resolved.
                    *lock.lock() = true;
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
                            self.commit_hook(
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
        executable_cache: Arc<ExecutableStore<T::Key, X>>,
    ) -> Result<Vec<E::Output>, E::Error> {
        let _timer = PARALLEL_EXECUTION_SECONDS.start_timer();
        // Using parallel execution with 1 thread currently will not work as it
        // will only have a coordinator role but no workers for rolling commit.
        // Need to special case no roles (commit hook by thread itself) to run
        // w. concurrency_level = 1 for some reason.
        assert!(self.concurrency_level > 1, "Must use sequential execution");

        let versioned_cache = MVHashMap::new(executable_cache);

        if signature_verified_block.is_empty() {
            return Ok(vec![]);
        }

        let num_txns = signature_verified_block.len() as u32;
        let last_input_output = TxnLastInputOutput::new(num_txns);
        let scheduler = Scheduler::new(num_txns);

        let mut roles: Vec<CommitRole> = vec![];
        let mut senders = Vec::with_capacity(self.concurrency_level - 1);
        for _ in 0..(self.concurrency_level - 1) {
            let (tx, rx) = mpsc::channel();
            roles.push(CommitRole::Worker(rx));
            senders.push(tx);
        }
        // Add the coordinator role. Coordinator is responsible for committing
        // indices and assigning post-commit work per index to other workers.
        // Note: It is important that the Coordinator is the first thread that
        // picks up a role will be a coordinator. Hence, if multiple parallel
        // executors are running concurrently, they will all havean active coordinator.
        roles.push(CommitRole::Coordinator(senders, 0));

        let timer = RAYON_EXECUTION_SECONDS.start_timer();
        RAYON_EXEC_POOL.scope(|s| {
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

        let (mv_data_cache, mv_code_cache) = RAYON_EXEC_POOL.install(|| versioned_cache.take());

        RAYON_EXEC_POOL.spawn(move || {
            // Explicit async drops.
            drop(last_input_output);
            drop(scheduler);
            drop(mv_data_cache);
            drop(mv_code_cache);
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
        signature_verified_block: &[T],
        base_view: &S,
    ) -> Result<Vec<E::Output>, E::Error> {
        let num_txns = signature_verified_block.len();
        let executor = E::init(executor_arguments);
        let data_map = UnsyncMap::default();

        let mut ret = Vec::with_capacity(num_txns);
        for (idx, txn) in signature_verified_block.iter().enumerate() {
            let res = executor.execute_transaction(
                &LatestView::<T, S, X>::new_unsync_view(base_view, &data_map, idx as TxnIndex),
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
                        data_map.insert(ap, write_op);
                    }
                    ret.push(output);
                },
                ExecutionStatus::Abort(err) => {
                    // Record the status indicating abort.
                    return Err(Error::UserError(err));
                },
            }

            if must_skip {
                break;
            }
        }

        info!(
            "Sequential execution used executable cache of size = {} bytes",
            data_map.executable_size(),
        );

        ret.resize_with(num_txns, E::Output::skip_output);
        Ok(ret)
    }

    pub fn execute_block(
        &self,
        executor_arguments: E::Argument,
        signature_verified_block: Vec<T>,
        base_view: &S,
        executable_cache: Arc<ExecutableStore<T::Key, X>>,
    ) -> Result<Vec<E::Output>, E::Error> {
        let mut ret = if self.concurrency_level > 1 {
            self.execute_transactions_parallel(
                executor_arguments,
                &signature_verified_block,
                base_view,
                executable_cache,
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

        RAYON_EXEC_POOL.spawn(move || {
            // Explicit async drops.
            drop(signature_verified_block);
        });

        ret
    }
}
