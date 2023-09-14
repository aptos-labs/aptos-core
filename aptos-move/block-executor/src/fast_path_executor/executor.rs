// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{marker::PhantomData, sync::Arc};

use crossbeam::atomic::AtomicCell;
use dashmap::DashMap;
use futures::channel::oneshot;
use itertools::Itertools;
use rayon::{
    iter::Either::{Left, Right},
    prelude::*,
    ThreadPool,
};
use thread_local::ThreadLocal;

use aptos_aggregator::delta_change_set::serialize;
use aptos_infallible::Mutex;
use aptos_mvhashmap::types::TxnIndex;
use aptos_state_view::TStateView;
use aptos_types::rayontools::ParExtendByRefTrait;
use aptos_types::write_set::WriteOp;
use move_core_types::account_address::AccountAddress;

use crate::{
    executor_traits::{BlockExecutor, BlockExecutorBase, HintedBlockExecutor},
    fast_path_executor::{
        reservation_table::{
            DashMapReservationTable, OptimisticDashMapReservationTable, ReservationTable,
        },
        view::{DashMapStateView, EmptyStateView, ReadSetCapturingStateView, WritableStateView},
    },
    task::{ExecutionStatus, ExecutorTask, Transaction, TransactionOutput},
    transaction_hints::TransactionWithHints,
};
use crate::errors::Error;
use crate::fast_path_executor::stats::{ExecutorStats, FastPathStats, WorkerStats};

#[allow(dead_code)] // TODO: handle `maybe_block_gas_limit` properly
pub struct FastPathBlockExecutor<T: Transaction, E, FB> {
    // TODO: consider getting rid of a fixed batch size.
    batch_size: usize,

    // TODO: maybe remove this parameter and expect caller to install the thread pool they want?
    executor_thread_pool: Arc<ThreadPool>,
    maybe_block_gas_limit: Option<u64>,

    // Mutex is used to avoid having to require FB to implement `Sync`
    fallback: Mutex<FB>,

    phantom: PhantomData<(T, E)>,
}

impl<T, E, FB> FastPathBlockExecutor<T, E, FB>
where
    T: Transaction + Send + Sync,
    E: ExecutorTask<Txn = T> + Send + Sync,
    E::Output: Send,
    FB: Send + HintedBlockExecutor<
        HintedTxn = TransactionWithHints<T>,
        Txn = T,
        ExecutorTask = E,
        Error = Error<E::Error>,
    >,
{
    pub fn new(
        batch_size: usize,
        executor_thread_pool: Arc<ThreadPool>,
        maybe_block_gas_limit: Option<u64>,
        fallback: FB,
    ) -> Self {
        Self {
            batch_size,
            executor_thread_pool,
            maybe_block_gas_limit,
            fallback: Mutex::new(fallback),
            phantom: PhantomData,
        }
    }

    fn execute_transaction(
        worker_state: &WorkerState<E>,
        txn: T,
        txn_idx: TxnIndex,
        state_view: &impl TStateView<Key = T::Key>,
        aggregators_snapshot: &impl WritableStateView<Key = T::Key, Value = T::Value>,
        write_reservations: &impl ReservationTable<T::Key>,
        delta_reservations: &impl ReservationTable<T::Key>,
        sequence_numbers: &DashMap<AccountAddress, Option<u64>>,
    ) -> Result<TxnExecutionInfo<E>, Error<E::Error>> {
        // NB: if we don't find the relevant sequence number in the map, there may
        // still be other transactions with smaller sequence number in the same batch.
        // However, source order will be preserved as two transactions from the same
        // source account cannot be committed in the same batch as there is always
        // a read-write conflict between them.
        if let Some((account, sn)) = txn.sender_and_sequence_number() {
            if let Some(opt_last_sn) = sequence_numbers.get(&account) {
                if opt_last_sn.is_none() || opt_last_sn.unwrap() + 1 != sn {
                    worker_state.stats.discard_reasons.add_sequence_number();
                    return Ok(TxnExecutionInfo {
                        transaction: txn,
                        read_set: vec![],
                        output: None,
                        abort: true,
                        // vm_execution_time: None,
                        txn_idx,
                        _phantom: PhantomData,
                    });
                }
            }
        }

        let read_set_capturing_view = ReadSetCapturingStateView::with_capacity(state_view, 10);

        let timer = std::time::Instant::now();
        let status = worker_state.executor_task.execute_transaction(
            &read_set_capturing_view,
            &txn,
            txn_idx,
            false
        );
        // let vm_execution_time = Some(timer.elapsed());

        let (output, abort) = match status {
            ExecutionStatus::Success(output) => (Some(output), false),
            ExecutionStatus::Abort(_) => {
                worker_state.stats.discard_reasons.add_vm_abort();
                (None, true)
            },
            // TODO: handle skip_rest properly
            ExecutionStatus::SkipRest(output) => (Some(output), false),
        };

        let txn_execution_info = TxnExecutionInfo {
            transaction: txn,
            read_set: read_set_capturing_view.take_read_set(),
            output,
            abort,
            txn_idx,
            _phantom: PhantomData,
        };

        if let Some(write_set) = txn_execution_info.write_set() {
            for k in write_set {
                write_reservations.make_reservation(k, txn_idx);
            }
        }

        if let Some(delta_set) = txn_execution_info.delta_set() {
            for k in delta_set {
                // As aggregators can be highly contended, first check if the snapshot
                // of this aggregator has already been made and only proceed to making
                // the snapshot otherwise.
                // TODO: check if `expect` is justified here.
                if let None = aggregators_snapshot.get_state_value_u128(&k).expect("Failed to read an aggregator") {
                    let val = state_view
                        .get_state_value_u128(&k)
                        .expect("Failed to read an aggregator")
                        .expect("Writing to a non-existent aggregator");
                    aggregators_snapshot.write_u128(k.clone(), val);
                }

                delta_reservations.make_reservation(k, txn_idx);
            }
        }

        Ok(txn_execution_info)
    }

    fn execute_transactions_in_parallel(
        worker_states: &ThreadLocal<WorkerState<E>>,
        executor_arguments: &E::Argument,
        transactions: impl IntoParallelIterator<Item = (usize, T)>,
        state_view: &(impl TStateView<Key = T::Key> + Sync),
        aggregators_snapshot: &(impl WritableStateView<Key = T::Key, Value = T::Value> + Sync),
        write_reservations: &(impl ReservationTable<T::Key> + Sync),
        delta_reservations: &(impl ReservationTable<T::Key> + Sync),
        sequence_numbers: &DashMap<AccountAddress, Option<u64>>,
    ) -> Result<Vec<TxnExecutionInfo<E>>, Error<E::Error>> {
        transactions
            .into_par_iter()
            .map_init(
                || worker_states.get_or(|| WorkerState::new(*executor_arguments)),
                |&mut worker_state, (txn_idx, txn)| {
                    Self::execute_transaction(
                        worker_state,
                        txn,
                        txn_idx as TxnIndex,
                        state_view,
                        aggregators_snapshot,
                        write_reservations,
                        delta_reservations,
                        sequence_numbers,
                    )
                },
            )
            .collect()
    }

    fn validate_transaction(
        worker_state: &WorkerState<E>,
        execution_info: &TxnExecutionInfo<E>,
        write_reservations: &(impl ReservationTable<T::Key> + Sync),
        delta_reservations: &(impl ReservationTable<T::Key> + Sync),
    ) -> bool {
        if execution_info.abort {
            return false;
        }

        let txn_idx = execution_info.txn_idx;
        for k in execution_info.read_set.iter() {
            // A transaction cannot be committed if it reads a location that is written by
            // a smaller-id transaction in the same batch.
            if let Some(reservation) = write_reservations.get_reservation(k) {
                if reservation < txn_idx {
                    worker_state.stats.discard_reasons.add_read_write_conflict();
                    return false;
                }
            }

            // A transaction cannot be committed if it reads an aggregator that was updated
            // by a smaller-id transaction in the same batch.
            if let Some(reservation) = delta_reservations.get_reservation(k) {
                if reservation < txn_idx {
                    worker_state.stats.discard_reasons.add_read_delta_conflict();
                    return false;
                }
            }
        }

        // TODO: later we will need to add aggregator overflow detection
        true
    }

    fn apply_transaction_output(
        txn_output: &E::Output,
        state: &(impl WritableStateView<Key = T::Key, Value = T::Value> + Sync),
    ) -> Result<(), Error<E::Error>> {
        let txn_writes = (txn_output.resource_write_set().into_iter())
            .chain(txn_output.aggregator_v1_write_set());

        for (k, v) in txn_writes {
            state.write_value(k, v);
        }

        for (k, delta_op) in txn_output.aggregator_v1_delta_set() {
            // TODO: check if `expect` is justified here.
            state.apply_delta(&k, &delta_op).expect("Aggregator application error");
        }

        Ok(())
    }

    fn validate_and_apply_transactions_in_parallel(
        worker_states: &ThreadLocal<WorkerState<E>>,
        executor_arguments: &E::Argument,
        execution_results: Vec<TxnExecutionInfo<E>>,
        write_reservations: &(impl ReservationTable<T::Key> + Sync),
        delta_reservations: &(impl ReservationTable<T::Key> + Sync),
        sequence_numbers: &DashMap<AccountAddress, Option<u64>>,
        output_view: &(impl WritableStateView<Key = T::Key, Value = T::Value> + Sync),
        discarded_txns: &mut Vec<TransactionWithHints<T>>,
    ) -> Result<Vec<E::Output>, Error<E::Error>> {
        let error: AtomicCell<Option<Error<E::Error>>> = AtomicCell::new(None);

        let mut selected_txn_outputs = Vec::with_capacity(execution_results.len());

        (selected_txn_outputs.by_ref(), discarded_txns.by_ref()).par_extend(
            execution_results.into_par_iter().map_init(
                || worker_states.get_or(|| WorkerState::new(*executor_arguments)),
                |&mut worker_state, exec_info| {
                    let select = Self::validate_transaction(
                        worker_state,
                        &exec_info,
                        write_reservations,
                        delta_reservations,
                    );

                    if select {
                        if let Some((account, sn)) = exec_info.transaction.sender_and_sequence_number() {
                            sequence_numbers.insert(account, Some(sn));
                        }

                        let output = exec_info.output.unwrap();
                        let err = Self::apply_transaction_output(&output, output_view);

                        if let Err(e) = err {
                            error.store(Some(e));
                        }
                        Left(output)
                    } else {
                        // In the current implementation, if one transaction from an account is
                        // discarded, they all are. `None` acts as a marker of such "deactivated"
                        // account in this case.
                        if let Some((account, sn)) = exec_info.transaction.sender_and_sequence_number() {
                            sequence_numbers.insert(account, None);
                        }

                        let write_set = exec_info.write_set().map(Iterator::collect).unwrap_or(vec![]);
                        let delta_set = exec_info.delta_set().map(Iterator::collect).unwrap_or(vec![]);
                        let TxnExecutionInfo{ transaction, read_set, .. } = exec_info;

                        Right(TransactionWithHints { transaction, read_set, write_set, delta_set })
                    }
                },
            ),
        );

        if let Some(e) = error.take() {
            return Err(e);
        }

        Ok(selected_txn_outputs)
    }

    fn materialize_deltas(
        outputs_in_serialization_order: &Vec<E::Output>,
        base_view: impl TStateView<Key = T::Key>,
    ) -> Result<(), Error<E::Error>> {
        let updated_view = DashMapStateView::<_, T::Value, _>::new(base_view);

        for output in outputs_in_serialization_order {
            let delta_writes = output
                .aggregator_v1_delta_set()
                .into_iter()
                .map(|(k, delta)| {
                    // TODO: check if `expect` is justified here.
                    let new_value = updated_view.apply_delta(&k, &delta)
                        .expect("Delta application error");
                    Ok((k, WriteOp::Modification(serialize(&new_value))))
                })
                .collect::<Result<Vec<_>, Error<E::Error>>>()?;

            output.incorporate_delta_writes(delta_writes);
        }

        Ok(())
    }

    fn process_batch<Txns>(
        &self,
        transactions: Txns,
        worker_states: &ThreadLocal<WorkerState<E>>,
        executor_arguments: &E::Argument,
        writable_view: &(impl WritableStateView<Key = T::Key, Value = T::Value> + Sync),
        sequence_numbers: &DashMap<AccountAddress, Option<u64>>,
        discarded_txns: &mut Vec<TransactionWithHints<T>>,
        fast_path_stats: &mut FastPathStats,
    ) -> Result<oneshot::Receiver<Vec<E::Output>>, Error<E::Error>>
    where
        Txns: IntoParallelIterator<Item = (usize, T)>,
        Txns::Iter: IndexedParallelIterator,
    {
        let start = std::time::Instant::now();

        let transactions = transactions.into_par_iter();
        let batch_size = transactions.len();

        let write_reservations = DashMapReservationTable::with_capacity(4 * batch_size);
        // Aggregators are often highly contended. Using OptimisticDashMapReservationTable allows
        // avoiding having all transactions compete for the same write lock.
        let delta_reservations =
            OptimisticDashMapReservationTable::with_capacity(2 * batch_size);

        let aggregators_snapshot = DashMapStateView::new(EmptyStateView::new());

        let after_init = std::time::Instant::now();

        let execution_results = Self::execute_transactions_in_parallel(
            &worker_states,
            executor_arguments,
            transactions,
            &writable_view.as_state_view(),
            &aggregators_snapshot,
            &write_reservations,
            &delta_reservations,
            &sequence_numbers,
        )?;

        let after_execution = std::time::Instant::now();

        let selected_outputs = Self::validate_and_apply_transactions_in_parallel(
            &worker_states,
            executor_arguments,
            execution_results,
            &write_reservations,
            &delta_reservations,
            &sequence_numbers,
            writable_view,
            discarded_txns,
        )?;
        let _selected_txns_count = selected_outputs.len();

        let after_validation = std::time::Instant::now();

        let (done_tx, done_rx) = oneshot::channel::<Vec<E::Output>>();

        // Materialize deltas asynchronously, off the critical path.
        // Take the ownership of the selected outputs for the materialization and then
        // return it via the channel.
        rayon::spawn(move || {
            // TODO: potentially handle the materialization errors.
            Self::materialize_deltas(
                &selected_outputs,
                aggregators_snapshot,
            )
            .expect("Failed to materialize aggregators");

            done_tx.send(selected_outputs).unwrap();
        });

        let after_spawn = std::time::Instant::now();

        fast_path_stats.total_batch_processing_time += after_spawn - start;
        fast_path_stats.batch_init_time += after_init - start;
        fast_path_stats.execution_time += after_execution - after_init;
        fast_path_stats.validation_time += after_validation - after_execution;
        fast_path_stats.materialization_task_spawn_time += after_spawn - after_validation;

        // println!("Batch processing stats:");
        // println!("\tBatch size: {}", batch_size);
        // println!("\tSelected txns: {}", _selected_txns_count);
        // println!("\tTime breakdown:");
        // println!("\t\tTotal: {:?}", after_spawn - start);
        // println!("\t\tInit: {:?}", after_init - start);
        // println!("\t\tExecution: {:?}", after_execution - after_init);
        // println!("\t\tValidation: {:?}", after_validation - after_execution);
        // println!("\t\tMaterialization task spawn: {:?}", after_spawn - after_validation);
        // println!("\tTotal time per transaction: {:?}", (after_spawn - start) / batch_size as u32);
        // println!("\tTotal time per selected transaction: {:?}", (after_spawn - start) / _selected_txns_count as u32);

        Ok(done_rx)
    }

    /// This function should be executed in the context of the thread pool via
    /// `self.executor_thread_pool.install`.
    fn execute_block_impl(
        &self,
        executor_arguments: E::Argument,
        mut signature_verified_block: Vec<T>,
        base_view: &(impl TStateView<Key = T::Key> + Sync),
    ) -> Result<Vec<E::Output>, Error<E::Error>> {
        // Create the data structure to store the statistics for observability.
        let mut executor_stats = ExecutorStats::default();

        // Start the timers (one for the whole function and one for the current stage).
        let function_start = std::time::Instant::now();
        let stage_timer = std::time::Instant::now();

        let block_size = signature_verified_block.len();
        let batch_count = (block_size + self.batch_size - 1) / self.batch_size;

        // The last transaction must not be reordered, so, as a simple solution,
        // we ignore it in the fast path and send it as the last transaction in the fallback
        // executor input.
        let last_txn = signature_verified_block.pop().unwrap();

        // Initialize the data structures that are used across batches.
        let state = DashMapStateView::with_capacity(base_view, block_size);
        // `+ 1` is for outputs yielded by the fallback.
        let mut committed_outputs_receivers = Vec::with_capacity(batch_count + 1);
        let mut discarded_txns = Vec::<TransactionWithHints<T>>::with_capacity(block_size);
        let sequence_numbers = DashMap::with_capacity(block_size);
        let mut worker_states = ThreadLocal::new();

        // Store the stage stats and reset the stage timer.
        executor_stats.time_stats.init = stage_timer.elapsed();
        let stage_timer = std::time::Instant::now();

        // Execute the fast path.
        let mut transactions = signature_verified_block.into_iter().enumerate();
        let mut first_batch = true;
        while transactions.len() != 0 {
            let batch_size = if first_batch {
                // The first transaction is special and conflicts with every other transaction.
                // As a simple albeit inefficient solution, we execute it in a separate batch.
                1
            } else {
                self.batch_size
            };

            // FIXME: this collect shouldn't be necessary.
            let batch_transactions = transactions.by_ref().take(batch_size).collect_vec();

            let committed_output_receiver = self.process_batch(
                batch_transactions,
                &worker_states,
                &executor_arguments,
                &state,
                &sequence_numbers,
                &mut discarded_txns,
                &mut executor_stats.fast_path_stats,
            )?;

            committed_outputs_receivers.push(committed_output_receiver);

            first_batch = false;
        }

        // Aggregate the stats from all worker threads.
        for worker_state in worker_states.iter_mut() {
            executor_stats.fast_path_stats += &worker_state.stats;
        }

        // Add the last transaction to the discarded transactions so that
        // it is executed by the fallback executor.
        discarded_txns.push(TransactionWithHints {
            transaction: last_txn,
            read_set: vec![],
            write_set: vec![],
            delta_set: vec![],
        });

        // TODO: consider doing it asynchronously, off the critical path.
        drop(worker_states);
        drop(sequence_numbers);

        // Store the stage stats and reset the stage timer.
        executor_stats.time_stats.fast_path = stage_timer.elapsed();
        executor_stats.total_txn_count = block_size;
        executor_stats.fallback_txn_count = discarded_txns.len();
        let stage_timer = std::time::Instant::now();

        // Execute the discarded transactions with the fallback executor.
        let fallback_outputs = (self.fallback.lock())
            .execute_block_hinted(executor_arguments, discarded_txns, &state)?;

        // Store the stage stats and reset the stage timer.
        executor_stats.time_stats.fallback = stage_timer.elapsed();
        let stage_timer = std::time::Instant::now();

        // Wait for all materialization tasks to finish.
        let mut committed_outputs: Vec<Vec<E::Output>> = committed_outputs_receivers
            .into_iter()
            .map(|mut receiver| loop {
                if let Some(data) = receiver.try_recv().unwrap() {
                    return data;
                }
            })
            .collect();

        // Store the stage stats and reset the stage timer.
        executor_stats.time_stats.wait = stage_timer.elapsed();
        let stage_timer = std::time::Instant::now();

        // Reconstruct the final output.
        committed_outputs.push(fallback_outputs);
        let final_output: Vec<_> = committed_outputs.into_par_iter().flatten().collect();

        // Store the stage stats and the total time.
        executor_stats.time_stats.final_output_reconstruction = stage_timer.elapsed();
        executor_stats.time_stats.total = function_start.elapsed();

        println!("{}", executor_stats);

        assert_eq!(final_output.len(), block_size);
        Ok(final_output)
    }
}

struct WorkerState<E> {
    executor_task: E,
    stats: WorkerStats,
}

impl<E: ExecutorTask> WorkerState<E> {
    fn new(executor_arguments: E::Argument) -> Self {
        Self {
            executor_task: E::init(executor_arguments),
            stats: WorkerStats::default(),
        }
    }
}

struct TxnExecutionInfo<E: ExecutorTask> {
    transaction: E::Txn,
    read_set: Vec<<E::Txn as Transaction>::Key>,
    output: Option<E::Output>,
    abort: bool,
    txn_idx: TxnIndex,

    _phantom: PhantomData<E>,
}

impl<E: ExecutorTask> TxnExecutionInfo<E> {
    fn write_set(&self) -> Option<impl Iterator<Item = <E::Txn as Transaction>::Key>> {
        let output = self.output.as_ref()?;
        let write_set = (output.resource_write_set().into_keys())
            .chain(output.aggregator_v1_write_set().into_keys());
        Some(write_set)
    }

    fn delta_set(&self) -> Option<impl Iterator<Item = <E::Txn as Transaction>::Key>> {
        let output = self.output.as_ref()?;
        Some(output.aggregator_v1_delta_set().into_keys())
    }
}

impl<T, E, FB> BlockExecutorBase for FastPathBlockExecutor<T, E, FB>
where
    T: Transaction,
    E: ExecutorTask<Txn = T> + Send,
{
    type Error = Error<E::Error>;
    type ExecutorTask = E;
    type Txn = T;
}

impl<T, E, FB> BlockExecutor for FastPathBlockExecutor<T, E, FB>
where
    T: Transaction,
    E: ExecutorTask<Txn = T> + Send,
    FB: Send + HintedBlockExecutor<
        HintedTxn = TransactionWithHints<T>,
        Txn = T,
        ExecutorTask = E,
        Error = Error<E::Error>,
    >,
{
    fn execute_block<S: TStateView<Key = T::Key> + Sync>(
        &self,
        executor_arguments: E::Argument,
        signature_verified_block: Vec<T>,
        base_view: &S,
    ) -> Result<Vec<E::Output>, Error<E::Error>> {
        self.executor_thread_pool.install(|| {
            self.execute_block_impl(executor_arguments, signature_verified_block, base_view)
        })
    }
}
