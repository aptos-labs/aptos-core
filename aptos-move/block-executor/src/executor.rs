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
    output_delta_resolver::OutputDeltaResolver,
    scheduler::{Scheduler, SchedulerTask, Wave},
    task::{ExecutionStatus, ExecutorTask, Transaction, TransactionOutput},
    txn_last_input_output::TxnLastInputOutput,
    view::{LatestView, MVHashMapView},
};
use aptos_logger::debug;
use aptos_mvhashmap::{
    types::{MVDataError, MVDataOutput, TxnIndex, Version},
    MVHashMap,
};
use aptos_state_view::TStateView;
use aptos_types::{
    executable::ExecutableTestType, // TODO: fix up with the proper generics.
    write_set::WriteOp,
};
use aptos_vm_logging::{clear_speculative_txn_logs, init_speculative_logs};
use num_cpus;
use once_cell::sync::Lazy;
use std::{
    collections::btree_map::BTreeMap,
    marker::PhantomData,
    sync::atomic::{AtomicBool, Ordering},
};

pub static RAYON_EXEC_POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .thread_name(|index| format!("par_exec_{}", index))
        .build()
        .unwrap()
});

pub struct BlockExecutor<T, E, S> {
    // number of active concurrent tasks, corresponding to the maximum number of rayon
    // threads that may be concurrently participating in parallel execution.
    concurrency_level: usize,
    phantom: PhantomData<(T, E, S)>,
}

impl<T, E, S> BlockExecutor<T, E, S>
where
    T: Transaction,
    E: ExecutorTask<Txn = T>,
    S: TStateView<Key = T::Key> + Sync,
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
        versioned_cache: &MVHashMap<T::Key, T::Value, ExecutableTestType>,
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
            &LatestView::<T, S>::new_mv_view(base_view, &speculative_view, idx_to_execute),
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

        last_input_output.record(idx_to_execute, speculative_view.take_reads(), result);
        scheduler.finish_execution(idx_to_execute, incarnation, updates_outside)
    }

    fn validate(
        &self,
        version_to_validate: Version,
        validation_wave: Wave,
        last_input_output: &TxnLastInputOutput<T::Key, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Value, ExecutableTestType>,
        scheduler: &Scheduler,
    ) -> SchedulerTask {
        use MVDataError::*;
        use MVDataOutput::*;

        let _timer = TASK_VALIDATE_SECONDS.start_timer();
        let (idx_to_validate, incarnation) = version_to_validate;
        let read_set = last_input_output
            .read_set(idx_to_validate)
            .expect("Prior read-set must be recorded");

        let valid = read_set.iter().all(|r| {
            match versioned_cache.fetch_data(r.path(), idx_to_validate) {
                Ok(Versioned(version, _)) => r.validate_version(version),
                Ok(Resolved(value)) => r.validate_resolved(value),
                Err(Dependency(_)) => false, // Dependency implies a validation failure.
                Err(Unresolved(delta)) => r.validate_unresolved(delta),
                Err(NotFound) => r.validate_storage(),
                // We successfully validate when read (again) results in a delta application
                // failure. If the failure is speculative, a later validation will fail due to
                // a read without this error. However, if the failure is real, passing
                // validation here allows to avoid infinitely looping and instead panic when
                // materializing deltas as writes in the final output preparation state. Panic
                // is also preferrable as it allows testing for this scenario.
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

    fn work_task_with_scope(
        &self,
        executor_arguments: &E::Argument,
        block: &[T],
        last_input_output: &TxnLastInputOutput<T::Key, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Value, ExecutableTestType>,
        scheduler: &Scheduler,
        base_view: &S,
        committing: bool,
    ) {
        // Make executor for each task. TODO: fast concurrent executor.
        let init_timer = VM_INIT_SECONDS.start_timer();
        let executor = E::init(*executor_arguments);
        drop(init_timer);

        let _timer = WORK_WITH_TASK_SECONDS.start_timer();
        let mut scheduler_task = SchedulerTask::NoTask;
        loop {
            // Only one thread try_commit to avoid contention.
            if committing {
                // Keep committing txns until there is no more that can be committed now.
                while let Some(txn_idx) = scheduler.try_commit() {
                    if txn_idx as usize + 1 == block.len() {
                        // Committed the last transaction / everything.
                        scheduler_task = SchedulerTask::Done;
                        break;
                    }
                }
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
    ) -> Result<Vec<(E::Output, Vec<(T::Key, WriteOp)>)>, E::Error> {
        let _timer = PARALLEL_EXECUTION_SECONDS.start_timer();
        assert!(self.concurrency_level > 1, "Must use sequential execution");

        let versioned_cache = MVHashMap::new(None);

        if signature_verified_block.is_empty() {
            return Ok(vec![]);
        }

        let num_txns = signature_verified_block.len() as u32;
        let last_input_output = TxnLastInputOutput::new(num_txns);
        let committing = AtomicBool::new(true);
        let scheduler = Scheduler::new(num_txns);

        let timer = RAYON_EXECUTION_SECONDS.start_timer();
        RAYON_EXEC_POOL.scope(|s| {
            for _ in 0..self.concurrency_level {
                s.spawn(|_| {
                    self.work_task_with_scope(
                        &executor_initial_arguments,
                        signature_verified_block,
                        &last_input_output,
                        &versioned_cache,
                        &scheduler,
                        base_view,
                        committing.swap(false, Ordering::SeqCst),
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

        RAYON_EXEC_POOL.spawn(move || {
            // Explicit async drops.
            drop(last_input_output);
            drop(scheduler);
        });

        match maybe_err {
            Some(err) => Err(err),
            None => {
                final_results.resize_with(num_txns, E::Output::skip_output);
                let (mv_data_cache, _mv_code_cache) = versioned_cache.take();
                let delta_resolver: OutputDeltaResolver<T> =
                    OutputDeltaResolver::new(mv_data_cache);
                // TODO: parallelize when necessary.
                Ok(final_results
                    .into_iter()
                    .zip(delta_resolver.resolve(base_view, num_txns).into_iter())
                    .collect())
            },
        }
    }

    pub(crate) fn execute_transactions_sequential(
        &self,
        executor_arguments: E::Argument,
        signature_verified_block: &[T],
        base_view: &S,
    ) -> Result<Vec<(E::Output, Vec<(T::Key, WriteOp)>)>, E::Error> {
        let num_txns = signature_verified_block.len();
        let executor = E::init(executor_arguments);
        let mut data_map = BTreeMap::new();

        let mut ret = Vec::with_capacity(num_txns);
        for (idx, txn) in signature_verified_block.iter().enumerate() {
            let res = executor.execute_transaction(
                &LatestView::<T, S>::new_btree_view(base_view, &data_map, idx as TxnIndex),
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

        ret.resize_with(num_txns, E::Output::skip_output);
        Ok(ret.into_iter().map(|out| (out, vec![])).collect())
    }

    pub fn execute_block(
        &self,
        executor_arguments: E::Argument,
        signature_verified_block: Vec<T>,
        base_view: &S,
    ) -> Result<Vec<(E::Output, Vec<(T::Key, WriteOp)>)>, E::Error> {
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

        RAYON_EXEC_POOL.spawn(move || {
            // Explicit async drops.
            drop(signature_verified_block);
        });

        ret
    }
}
