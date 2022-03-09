// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    errors::*,
    outcome_array::OutcomeArray,
    scheduler::{Scheduler, SchedulerTask, TaskGuard, TxnIndex, Version},
    task::{ExecutionStatus, ExecutorTask, Transaction, TransactionOutput},
    txn_last_input_output::{ReadDescriptor, TxnLastInputOutput},
};
use anyhow::{bail, Result as AResult};
use aptos_infallible::Mutex;
use mvhashmap::MVHashMap;
use num_cpus;
use rayon::{prelude::*, scope};
use std::{
    collections::HashSet,
    hash::Hash,
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::spawn,
};

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
    read_dependency: AtomicBool,
    captured_reads: Mutex<Vec<ReadDescriptor<K>>>,
}

impl<'a, K: PartialOrd + Send + Clone + Hash + Eq, V: Send + Sync> MVHashMapView<'a, K, V> {
    /// Drains the captured reads.
    pub fn take_reads(&self) -> Vec<ReadDescriptor<K>> {
        let mut reads = self.captured_reads.lock();
        std::mem::take(&mut reads)
    }

    /// Captures a read from the VM execution.
    pub fn read(&self, key: &K) -> AResult<Option<Arc<V>>> {
        loop {
            match self.versioned_map.read(key, self.txn_idx) {
                Ok((version, v)) => {
                    let (txn_idx, incarnation) = version;
                    self.captured_reads.lock().push(ReadDescriptor::from(
                        key.clone(),
                        txn_idx,
                        incarnation,
                    ));
                    return Ok(Some(v));
                }
                Err(None) => {
                    self.captured_reads
                        .lock()
                        .push(ReadDescriptor::from_storage(key.clone()));
                    return Ok(None);
                }
                Err(Some(dep_idx)) => {
                    // Don't start execution transaction `self.txn_idx` until `dep_idx` is computed.
                    if self.scheduler.try_add_dependency(self.txn_idx, dep_idx) {
                        // dep_idx is already executed, push `self.txn_idx` to ready queue.
                        self.read_dependency.store(true, Ordering::Relaxed);
                        bail!("Read dependency is not computed, retry later")
                    } else {
                        // Re-read, as the dependency got resolved.
                        continue;
                    }
                }
            };
        }
    }

    /// Return txn_idx associated with the MVHashMapView
    pub fn txn_idx(&self) -> TxnIndex {
        self.txn_idx
    }

    /// Return whether a read dependency was encountered during VM execution.
    pub fn read_dependency(&self) -> bool {
        self.read_dependency.load(Ordering::Relaxed)
    }
}

pub struct ParallelTransactionExecutor<T: Transaction, E: ExecutorTask> {
    num_cpus: usize,
    phantom: PhantomData<(T, E)>,
}

impl<T, E> ParallelTransactionExecutor<T, E>
where
    T: Transaction,
    E: ExecutorTask<T = T>,
{
    pub fn new() -> Self {
        Self {
            num_cpus: num_cpus::get(),
            phantom: PhantomData,
        }
    }

    pub fn execute<'a>(
        &self,
        version_to_execute: Version,
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
        let (idx_to_execute, incarnation) = version_to_execute;
        let txn = &signature_verified_block[idx_to_execute];

        // An optimization to pre-check that there are no read dependencies once prior read-set
        // is available, to avoid an execution that will likely be discarded due to the dependency.
        // TODO (issue 10180): remove once we have a way to suspend VM execution (so partial
        // execution would not be discarded).
        if let Some(read_set) = last_input_output.read_set(idx_to_execute) {
            if read_set.iter().any(
                |r| match versioned_data_cache.read(r.path(), idx_to_execute) {
                    Err(Some(dep_idx)) => scheduler.try_add_dependency(idx_to_execute, dep_idx),
                    Ok(_) | Err(None) => false,
                },
            ) {
                // Transaction has a read dependency. Was not executed and thus nothing to validate.
                return SchedulerTask::NoTask;
            }
        }

        let state_view = MVHashMapView {
            versioned_map: versioned_data_cache,
            txn_idx: idx_to_execute,
            scheduler,
            read_dependency: AtomicBool::new(false),
            captured_reads: Mutex::new(Vec::new()),
        };

        // VM execution.
        let execute_result = executor.execute_transaction(&state_view, txn);

        if state_view.read_dependency() {
            // Encountered and already handled (added to Scheduler) a read dependency.
            return SchedulerTask::NoTask;
        }

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

    pub fn validate<'a>(
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
        let compute_cpus = self.num_cpus;
        let last_input_output = TxnLastInputOutput::new(num_txns);
        let scheduler = Scheduler::new(num_txns);

        scope(|s| {
            println!(
                "Launching {} threads to execute... total txns: {:?}",
                compute_cpus,
                scheduler.num_txn_to_execute(),
            );

            for _ in 0..(compute_cpus) {
                s.spawn(|_| {
                    // Make executor for each thread.
                    let executor = E::init(executor_initial_arguments);

                    let mut scheduler_task = SchedulerTask::NoTask;
                    loop {
                        scheduler_task = match scheduler_task {
                            SchedulerTask::ValidationTask(version_to_validate, guard) => self
                                .validate(
                                    version_to_validate,
                                    guard,
                                    &last_input_output,
                                    &versioned_data_cache,
                                    &scheduler,
                                ),
                            SchedulerTask::ExecutionTask(version_to_execute, guard) => self
                                .execute(
                                    version_to_execute,
                                    guard,
                                    &signature_verified_block,
                                    &last_input_output,
                                    &versioned_data_cache,
                                    &scheduler,
                                    &executor,
                                ),
                            SchedulerTask::NoTask => scheduler.next_task(),
                            SchedulerTask::Done => break,
                        }
                    }
                });
            }
        });

        // Extract outputs in parallel
        let valid_results_size = scheduler.num_txn_to_execute();
        let chunk_size = (valid_results_size + 4 * compute_cpus - 1) / (4 * compute_cpus);
        (0..valid_results_size)
            .collect::<Vec<TxnIndex>>()
            .par_chunks(chunk_size)
            .map(|chunk| {
                for idx in chunk.iter() {
                    outcomes.set_result(*idx, last_input_output.take_output(*idx));
                }
            })
            .collect::<()>();

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
