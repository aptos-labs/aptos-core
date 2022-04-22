// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    errors::*,
    outcome_array::OutcomeArray,
    scheduler::{Scheduler, SchedulerTask, TaskGuard, TxnIndex, Version},
    task::{ExecutionStatus, ExecutorTask, Transaction, TransactionOutput},
    txn_last_input_output::{ReadDescriptor, TxnLastInputOutput},
};
use aptos_infallible::Mutex;
use mvhashmap::MVHashMap;
use num_cpus;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use std::{collections::HashSet, hash::Hash, marker::PhantomData, sync::Arc, thread::spawn};

static RAYON_EXEC_POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .build()
        .unwrap()
});

/// A struct that is always used by a single thread performing an execution task. The struct is
/// passed to the VM and acts as a proxy to resolve reads first in the shared multi-version
/// data-structure. It also allows the caller to track the read-set and any dependencies.
///
/// TODO(issue 10177): MvHashMapView currently needs to be sync due to trait bounds, but should
/// not be. In this case, the read_dependency member can have a RefCell<bool> type and the
/// captured_reads member can have RefCell<Vec<ReadDescriptor<K>>> type.
pub struct MVHashMapView<'a, K, V> {
    versioned_map: &'a MVHashMap<K, V>,
    txn_idx: TxnIndex,
    scheduler: &'a Scheduler,
    captured_reads: Mutex<Vec<ReadDescriptor<K>>>,
}

impl<'a, K: PartialOrd + Send + Clone + Hash + Eq, V: Send + Sync> MVHashMapView<'a, K, V> {
    /// Drains the captured reads.
    pub fn take_reads(&self) -> Vec<ReadDescriptor<K>> {
        let mut reads = self.captured_reads.lock();
        std::mem::take(&mut reads)
    }

    /// Captures a read from the VM execution.
    pub fn read(&self, key: &K) -> Option<Arc<V>> {
        loop {
            match self.versioned_map.read(key, self.txn_idx) {
                Ok((version, v)) => {
                    let (txn_idx, incarnation) = version;
                    self.captured_reads.lock().push(ReadDescriptor::from(
                        key.clone(),
                        txn_idx,
                        incarnation,
                    ));
                    return Some(v);
                }
                Err(None) => {
                    self.captured_reads
                        .lock()
                        .push(ReadDescriptor::from_storage(key.clone()));
                    return None;
                }
                Err(Some(dep_idx)) => {
                    // `self.txn_idx` estimated to depend on a write from `dep_idx`.
                    match self.scheduler.wait_for_dependency(self.txn_idx, dep_idx) {
                        Some(dep_condition) => {
                            // Wait on a condition variable correpsonding to the encountered
                            // read dependency. Once the dep_idx finishes re-execution, scheduler
                            // will mark the dependency as resolved, and then the txn_idx will be
                            // scheduled for re-execution, which will re-awaken cvar here.
                            // A deadlock is not possible due to these condition variables:
                            // suppose all threads are waiting on read dependency, and consider
                            // one with lowest txn_idx. It observed a dependency, so some thread
                            // aborted dep_idx. If that abort returned execution task, by
                            // minimality (lower transactions aren't waiting), that thread would
                            // finish execution unblock txn_idx, contradiction. Otherwise,
                            // execution_idx in scheduler was lower at a time when at least the
                            // thread that aborted dep_idx was alive, and again, since lower txns
                            // than txn_idx are not blocked, so the execution of dep_idx will
                            // eventually finish and lead to unblocking txn_idx, contradiction.
                            let (lock, cvar) = &*dep_condition;
                            let mut dep_resolved = lock.lock();
                            while !*dep_resolved {
                                dep_resolved = cvar.wait(dep_resolved).unwrap();
                            }
                        }
                        None => continue,
                    }
                }
            };
        }
    }

    /// Return txn_idx associated with the MVHashMapView
    pub fn txn_idx(&self) -> TxnIndex {
        self.txn_idx
    }
}

pub struct ParallelTransactionExecutor<T: Transaction, E: ExecutorTask> {
    // number of active concurrent tasks, corresponding to the maximum number of rayon
    // threads that may be concurrently participating in parallel execution.
    concurrency_level: usize,
    phantom: PhantomData<(T, E)>,
}

impl<T, E> ParallelTransactionExecutor<T, E>
where
    T: Transaction,
    E: ExecutorTask<T = T>,
{
    /// The caller needs to ensure that concurrency_level > 1 (0 is illegal and 1 should
    /// be handled by sequential execution) and that concurrency_level <= num_cpus.
    pub fn new(concurrency_level: usize) -> Self {
        assert!(
            concurrency_level > 1 && concurrency_level <= num_cpus::get(),
            "Parallel execution concurrency level {} should be between 2 and number of CPUs",
            concurrency_level
        );
        Self {
            concurrency_level,
            phantom: PhantomData,
        }
    }

    fn execute<'a>(
        &self,
        version: Version,
        guard: TaskGuard<'a>,
        signature_verified_block: &[T],
        last_input_output: &TxnLastInputOutput<
            <T as Transaction>::Key,
            <E as ExecutorTask>::Output,
            <E as ExecutorTask>::Error,
        >,
        versioned_data_cache: &MVHashMap<<T as Transaction>::Key, <T as Transaction>::Value>,
        scheduler: &'a Scheduler,
        executor: &E,
    ) -> SchedulerTask<'a> {
        let (idx_to_execute, incarnation) = version;
        let txn = &signature_verified_block[idx_to_execute];

        let state_view = MVHashMapView {
            versioned_map: versioned_data_cache,
            txn_idx: idx_to_execute,
            scheduler,
            captured_reads: Mutex::new(Vec::new()),
        };

        // VM execution.
        let execute_result = executor.execute_transaction(&state_view, txn);
        let mut prev_write_set: HashSet<T::Key> = last_input_output.write_set(idx_to_execute);

        // For tracking whether the recent execution wrote outside of the previous write set.
        let mut writes_outside = false;
        let mut apply_writes = |output: &<E as ExecutorTask>::Output| {
            let write_version = (idx_to_execute, incarnation);
            for (k, v) in output.get_writes().into_iter() {
                if !prev_write_set.remove(&k) {
                    writes_outside = true
                }
                versioned_data_cache.write(&k, write_version, v);
            }
        };

        let result = match execute_result {
            ExecutionStatus::Success(output) => {
                // Commit the side effects to the versioned_data_cache.
                apply_writes(&output);
                ExecutionStatus::Success(output)
            }
            ExecutionStatus::SkipRest(output) => {
                // Commit and skip the rest of the transactions.
                apply_writes(&output);
                scheduler.set_stop_idx(idx_to_execute + 1);
                ExecutionStatus::SkipRest(output)
            }
            ExecutionStatus::Abort(err) => {
                // Abort the execution with user defined error.
                scheduler.set_stop_idx(idx_to_execute + 1);
                ExecutionStatus::Abort(Error::UserError(err))
            }
        };

        // Remove entries from previous write set that were not overwritten.
        for k in &prev_write_set {
            versioned_data_cache.delete(k, idx_to_execute);
        }

        last_input_output.record(idx_to_execute, state_view.take_reads(), result);
        scheduler.finish_execution(idx_to_execute, incarnation, writes_outside, guard)
    }

    fn validate<'a>(
        &self,
        version_to_validate: Version,
        guard: TaskGuard<'a>,
        last_input_output: &TxnLastInputOutput<
            <T as Transaction>::Key,
            <E as ExecutorTask>::Output,
            <E as ExecutorTask>::Error,
        >,
        versioned_data_cache: &MVHashMap<<T as Transaction>::Key, <T as Transaction>::Value>,
        scheduler: &'a Scheduler,
    ) -> SchedulerTask<'a> {
        let (idx_to_validate, incarnation) = version_to_validate;
        let read_set = last_input_output
            .read_set(idx_to_validate)
            .expect("Prior read-set must be recorded");

        let valid = read_set.iter().all(|r| {
            match versioned_data_cache.read(r.path(), idx_to_validate) {
                Ok((version, _)) => r.validate_version(version),
                Err(Some(_)) => false, // Dependency implies a validation failure.
                Err(None) => r.validate_storage(),
            }
        });

        let aborted = !valid && scheduler.try_abort(idx_to_validate, incarnation);

        if aborted {
            // Not valid and successfully aborted, mark the latest write-set as estimates.
            for k in &last_input_output.write_set(idx_to_validate) {
                versioned_data_cache.mark_estimate(k, idx_to_validate);
            }

            scheduler.finish_abort(idx_to_validate, incarnation, guard)
        } else {
            SchedulerTask::NoTask
        }
    }

    fn work_task_with_scope(
        &self,
        executor_arguments: &E::Argument,
        block: &[T],
        last_input_output: &TxnLastInputOutput<
            <T as Transaction>::Key,
            <E as ExecutorTask>::Output,
            <E as ExecutorTask>::Error,
        >,
        versioned_data_cache: &MVHashMap<<T as Transaction>::Key, <T as Transaction>::Value>,
        scheduler: &Scheduler,
    ) {
        // Make executor for each task. TODO: fast concurrent executor.
        let executor = E::init(*executor_arguments);

        let mut scheduler_task = SchedulerTask::NoTask;
        loop {
            scheduler_task = match scheduler_task {
                SchedulerTask::ValidationTask(version_to_validate, guard) => self.validate(
                    version_to_validate,
                    guard,
                    last_input_output,
                    versioned_data_cache,
                    scheduler,
                ),
                SchedulerTask::ExecutionTask(version_to_execute, None, guard) => self.execute(
                    version_to_execute,
                    guard,
                    block,
                    last_input_output,
                    versioned_data_cache,
                    scheduler,
                    &executor,
                ),
                SchedulerTask::ExecutionTask(_, Some(condvar), _guard) => {
                    let (lock, cvar) = &*condvar;
                    // Mark dependency resolved.
                    *lock.lock() = true;
                    // Wake up the process waiting for dependency.
                    cvar.notify_one();

                    SchedulerTask::NoTask
                }
                SchedulerTask::NoTask => scheduler.next_task(),
                SchedulerTask::Done => {
                    break;
                }
            }
        }
    }

    pub fn execute_transactions_parallel(
        &self,
        executor_initial_arguments: E::Argument,
        signature_verified_block: Vec<T>,
    ) -> Result<Vec<E::Output>, E::Error> {
        if signature_verified_block.is_empty() {
            return Ok(vec![]);
        }

        let num_txns = signature_verified_block.len();
        let versioned_data_cache = MVHashMap::new();
        let outcomes = OutcomeArray::new(num_txns);
        let last_input_output = TxnLastInputOutput::new(num_txns);
        let scheduler = Scheduler::new(num_txns);

        RAYON_EXEC_POOL.scope(|s| {
            for _ in 0..self.concurrency_level {
                s.spawn(|_| {
                    self.work_task_with_scope(
                        &executor_initial_arguments,
                        &signature_verified_block,
                        &last_input_output,
                        &versioned_data_cache,
                        &scheduler,
                    );
                });
            }
        });

        // Extract outputs in parallel.
        let valid_results_size = scheduler.num_txn_to_execute();
        let chunk_size =
            (valid_results_size + 4 * self.concurrency_level - 1) / (4 * self.concurrency_level);
        RAYON_EXEC_POOL.install(|| {
            (0..valid_results_size)
                .collect::<Vec<TxnIndex>>()
                .par_chunks(chunk_size)
                .map(|chunk| {
                    for idx in chunk.iter() {
                        outcomes.set_result(*idx, last_input_output.take_output(*idx));
                    }
                })
                .collect::<()>();
        });

        spawn(move || {
            // Explicit async drops.
            drop(last_input_output);
            drop(signature_verified_block);
            drop(versioned_data_cache);
            drop(scheduler);
        });
        outcomes.get_all_results(valid_results_size)
    }
}
