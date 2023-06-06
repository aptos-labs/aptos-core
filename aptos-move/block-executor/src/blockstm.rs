// Copyright © Aptos Foundation

use std::collections::{BTreeMap, BTreeSet};
use std::marker::PhantomData;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use rayon::ThreadPool;
use aptos_logger::info;
use aptos_mvhashmap::MVHashMap;
use aptos_mvhashmap::types::MVDataError::{Dependency, Unresolved};
use aptos_mvhashmap::types::MVDataOutput::{Resolved, Versioned};
use aptos_mvhashmap::types::{TxnIndex, Version};
use aptos_state_view::TStateView;
use aptos_types::executable::ExecutableTestType;
use aptos_types::write_set::WriteOp;
use aptos_vm_logging::{clear_speculative_txn_logs};
use crate::{
    counters,
    counters::{TASK_EXECUTE_SECONDS, TASK_VALIDATE_SECONDS, VM_INIT_SECONDS, WORK_WITH_TASK_SECONDS}
};
use crate::errors::Error;
use crate::scheduler::{DependencyStatus, Scheduler, SchedulerTask, Wave};
use crate::task::{ExecutionStatus, ExecutorTask, Transaction, TransactionOutput};
use crate::txn_last_input_output::TxnLastInputOutput;
use crate::view::{LatestView, MVHashMapView};
use aptos_aggregator::delta_change_set::{deserialize, serialize};

#[derive(Debug)]
enum CommitRole {
    Coordinator(Vec<Sender<TxnIndex>>),
    Worker(Receiver<TxnIndex>),
}

pub struct InteractiveBlockSTM<T: Transaction, E: ExecutorTask, S> {
    executor_initial_arguments: Arc<E::Argument>,
    concurrency_level: usize,
    executor_thread_pool: Arc<ThreadPool>,
    base_view: Arc<S>,
    txn_indices: BTreeSet<TxnIndex>,
    versioned_cache: Arc<MVHashMap<T::Key, T::Value, ExecutableTestType>>,
    scheduler: Arc<Scheduler>,
    last_input_output: Arc<TxnLastInputOutput<T::Key, E::Output, E::Error>>,
    txns: Arc<BTreeMap<TxnIndex, T>>,
    worker_finished_rxs: Vec<Receiver<()>>,
    maybe_gas_limit: Option<u64>,
    phantom: PhantomData<(T, E, S)>,
}

impl<T, E, S> InteractiveBlockSTM<T, E, S>
    where
        T: Transaction,
        E: ExecutorTask<Txn = T>,
        S: TStateView<Key = T::Key> + Send + Sync + 'static,
        <E as ExecutorTask>::Argument: 'static,
{
    pub fn new(
        executor_initial_arguments: Arc<E::Argument>,
        concurrency_level: usize,
        executor_thread_pool: Arc<ThreadPool>,
        base_view: Arc<S>,
        maybe_gas_limit: Option<u64>
    ) -> Self {
        Self {
            executor_initial_arguments,
            concurrency_level,
            executor_thread_pool,
            base_view,
            txn_indices: BTreeSet::new(),
            versioned_cache: Arc::new(MVHashMap::new(None)),
            scheduler: Arc::new(Scheduler::new()),
            last_input_output: Arc::new(TxnLastInputOutput::new()),
            txns: Arc::new(BTreeMap::new()),
            worker_finished_rxs: vec![],
            maybe_gas_limit,
            phantom: Default::default(),
        }
    }

    pub fn start(&mut self) {
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

        for i in 0..self.concurrency_level {
            let role = roles.pop().expect("Role must be set for all threads");
            let (worker_finished_tx, worker_finished_rx) = mpsc::channel();
            let maybe_gas_limit = self.maybe_gas_limit;
            self.worker_finished_rxs.push(worker_finished_rx);
            let last_input_output = self.last_input_output.clone();
            let executor_initial_arguments = self.executor_initial_arguments.clone();
            let txns = self.txns.clone();
            let versioned_cache = self.versioned_cache.clone();
            let scheduler= self.scheduler.clone();
            let base_view = self.base_view.clone();
            self.executor_thread_pool.spawn(move|| {
                Self::work_task_with_scope(
                    maybe_gas_limit,
                    executor_initial_arguments,
                    txns,
                    last_input_output,
                    versioned_cache,
                    scheduler,
                    base_view,
                    role,
                );
                worker_finished_tx.send(()).unwrap();
            });
        }
    }

    pub fn new_txn(&mut self, txn: &T) {
        todo!()
    }

    pub fn new_external_txn_output(&mut self, execution_status: ExecutionStatus<<E as ExecutorTask>::Output, <E as ExecutorTask>::Error>) {
        todo!()
    }

    pub fn end_of_txn_stream(&mut self) {
        todo!()
    }

    pub fn wait(&mut self) -> Result<BTreeMap<TxnIndex, E::Output>, E::Error> {
        for rx in self.worker_finished_rxs.iter() {
            rx.recv().unwrap();
        }

        // TODO: for large block sizes and many cores, extract outputs in parallel.
        let mut final_results = BTreeMap::new();

        let maybe_err = if self.last_input_output.module_publishing_may_race() {
            unimplemented!()
        } else {
            let mut ret = None;
            for &idx in self.txn_indices.iter() {
                match self.last_input_output.take_output(idx) {
                    ExecutionStatus::Success(t) => final_results.insert(idx, t),
                    ExecutionStatus::SkipRest(t) => {
                        final_results.insert(idx, t);
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

        //TODO: explicit async probably has to be back.

        match maybe_err {
            Some(err) => todo!(),
            None => {
                let p = match final_results.last_key_value() {
                    Some((&k, _)) => k + 1,
                    None => 0,
                };
                for &i in self.txn_indices.range(p..){
                    final_results.insert(i, E::Output::skip_output());
                }
                Ok(final_results)
            },
        }
    }

    fn work_task_with_scope(
        maybe_gas_limit: Option<u64>,
        executor_arguments: Arc<E::Argument>,
        block: Arc<BTreeMap<TxnIndex, T>>,
        last_input_output: Arc<TxnLastInputOutput<T::Key, E::Output, E::Error>>,
        versioned_cache: Arc<MVHashMap<T::Key, T::Value, ExecutableTestType>>,
        scheduler: Arc<Scheduler>,
        base_view: Arc<S>,
        role: CommitRole,
    ) {
        // Make executor for each task. TODO: fast concurrent executor.
        let init_timer = VM_INIT_SECONDS.start_timer();
        let executor = E::init(*executor_arguments);
        drop(init_timer);

        let committing = matches!(role, CommitRole::Coordinator(_));

        let _timer = WORK_WITH_TASK_SECONDS.start_timer();
        let mut scheduler_task = SchedulerTask::NoTask;
        let mut accumulated_gas = 0;
        let mut worker_idx = 0;
        loop {
            // Only one thread does try_commit to avoid contention.
            match &role {
                CommitRole::Coordinator(post_commit_txs) => {
                    Self::coordinator_commit_hook(
                        maybe_gas_limit,
                        scheduler.clone(),
                        post_commit_txs,
                        &mut worker_idx,
                        &mut accumulated_gas,
                        &mut scheduler_task,
                        last_input_output.clone(),
                    );
                },
                CommitRole::Worker(rx) => {
                    while let Ok(txn_idx) = rx.try_recv() {
                        Self::worker_commit_hook(
                            txn_idx,
                            versioned_cache.clone(),
                            last_input_output.clone(),
                            base_view.clone(),
                        );
                    }
                },
            }

            scheduler_task = match scheduler_task {
                SchedulerTask::ValidationTask(version_to_validate, wave) => Self::validate(
                    version_to_validate,
                    wave,
                    last_input_output.clone(),
                    versioned_cache.clone(),
                    scheduler.clone(),
                ),
                SchedulerTask::ExecutionTask(version_to_execute, None) => Self::execute(
                    version_to_execute,
                    block.clone(),
                    last_input_output.clone(),
                    versioned_cache.clone(),
                    scheduler.clone(),
                    &executor,
                    base_view.clone(),
                ),
                SchedulerTask::ExecutionTask(_, Some(condvar)) => {
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
                            Self::worker_commit_hook(
                                txn_idx,
                                versioned_cache.clone(),
                                last_input_output.clone(),
                                base_view.clone(),
                            );
                        }
                    }
                    break;
                },
            }
        }

    }

    pub fn validate(
        version_to_validate: Version,
        validation_wave: Wave,
        last_input_output: Arc<TxnLastInputOutput<T::Key, E::Output, E::Error>>,
        versioned_cache: Arc<MVHashMap<T::Key, T::Value, ExecutableTestType>>,
        scheduler: Arc<Scheduler>,
    ) -> SchedulerTask {
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

    fn execute(
        version: Version,
        signature_verified_block: Arc<BTreeMap<TxnIndex, T>>,
        last_input_output: Arc<TxnLastInputOutput<T::Key, E::Output, E::Error>>,
        versioned_cache: Arc<MVHashMap<T::Key, T::Value, ExecutableTestType>>,
        scheduler: Arc<Scheduler>,
        executor: &E,
        base_view: Arc<S>,
    ) -> SchedulerTask {
        let _timer = TASK_EXECUTE_SECONDS.start_timer();
        let (idx_to_execute, incarnation) = version;
        let txn = signature_verified_block.get(&idx_to_execute).unwrap();

        let speculative_view = MVHashMapView::new(versioned_cache.as_ref(), scheduler.as_ref());

        // VM execution.
        let latest_view = LatestView::<T, S>::new_mv_view(base_view.as_ref(), &speculative_view, idx_to_execute);
        let execute_result: ExecutionStatus<<E as ExecutorTask>::Output, <E as ExecutorTask>::Error> = executor.execute_transaction(
            &latest_view,
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

    fn coordinator_commit_hook(
        maybe_gas_limit: Option<u64>,
        scheduler: Arc<Scheduler>,
        post_commit_txs: &Vec<Sender<u32>>,
        worker_idx: &mut usize,
        accumulated_gas: &mut u64,
        scheduler_task: &mut SchedulerTask,
        last_input_output: Arc<TxnLastInputOutput<T::Key, E::Output, E::Error>>,
    ) {
        while let Some(txn_idx) = scheduler.try_commit() {
            post_commit_txs[*worker_idx]
                .send(txn_idx)
                .expect("Worker must be available");
            // Iterate round robin over workers to do commit_hook.
            *worker_idx = (*worker_idx + 1) % post_commit_txs.len();

            // Committed the last transaction, BlockSTM finishes execution.
            if scheduler.txn_index_right_after(txn_idx).is_none() && scheduler.no_more_txns {
                *scheduler_task = SchedulerTask::Done;

                counters::PARALLEL_PER_BLOCK_GAS.observe(*accumulated_gas as f64);
                counters::PARALLEL_PER_BLOCK_COMMITTED_TXNS.observe((txn_idx + 1) as f64);
                info!(
                    "[BlockSTM]: Parallel execution completed, all {} txns committed.",
                    scheduler.get_txn_local_position(txn_idx) + 1
                );
                break;
            }

            // For committed txns with Success status, calculate the accumulated gas.
            // For committed txns with Abort or SkipRest status, early halt BlockSTM.
            match last_input_output.gas_used(txn_idx) {
                Some(gas) => {
                    *accumulated_gas += gas;
                    counters::PARALLEL_PER_TXN_GAS.observe(gas as f64);
                },
                None => {
                    scheduler.halt();

                    counters::PARALLEL_PER_BLOCK_GAS.observe(*accumulated_gas as f64);
                    counters::PARALLEL_PER_BLOCK_COMMITTED_TXNS.observe((txn_idx + 1) as f64);
                    info!("[BlockSTM]: Parallel execution early halted due to Abort or SkipRest txn, {} txns committed.", scheduler.get_txn_local_position(txn_idx) + 1);
                    break;
                },
            };

            if let Some(per_block_gas_limit) = maybe_gas_limit {
                // When the accumulated gas of the committed txns exceeds PER_BLOCK_GAS_LIMIT, early halt BlockSTM.
                if *accumulated_gas >= per_block_gas_limit {
                    // Set the execution output status to be SkipRest, to skip the rest of the txns.
                    last_input_output.update_to_skip_rest(txn_idx);
                    scheduler.halt();

                    counters::PARALLEL_PER_BLOCK_GAS.observe(*accumulated_gas as f64);
                    counters::PARALLEL_PER_BLOCK_COMMITTED_TXNS.observe((txn_idx + 1) as f64);
                    counters::PARALLEL_EXCEED_PER_BLOCK_GAS_LIMIT_COUNT.inc();
                    info!("[BlockSTM]: Parallel execution early halted due to accumulated_gas {} >= PER_BLOCK_GAS_LIMIT {}, {} txns committed", *accumulated_gas, per_block_gas_limit, txn_idx);
                    break;
                }
            }

            // Remark: When early halting the BlockSTM, we have to make sure the current / new tasks
            // will be properly handled by the threads. For instance, it is possible that the committing
            // thread holds an execution task from the last iteration, and then early halts the BlockSTM
            // due to a txn execution abort. In this case, we cannot reset the scheduler_task of the
            // committing thread (to be Done), otherwise some other pending thread waiting for the execution
            // will be pending on read forever (since the halt logic let the execution task to wake up such
            // pending task).
        }
    }

    fn worker_commit_hook(
        txn_idx: TxnIndex,
        versioned_cache: Arc<MVHashMap<T::Key, T::Value, ExecutableTestType>>,
        last_input_output: Arc<TxnLastInputOutput<T::Key, E::Output, E::Error>>,
        base_view: Arc<S>,
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
    }
}
