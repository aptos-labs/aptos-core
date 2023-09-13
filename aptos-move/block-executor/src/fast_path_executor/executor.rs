// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{marker::PhantomData, sync::Arc};
use std::sync::atomic::{AtomicU32};
use std::sync::atomic::Ordering::SeqCst;

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
use aptos_types::write_set::WriteOp;
use aptos_types::rayontools::ParExtendByRefTrait;
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

#[derive(Default, Debug)]
pub struct FastPathStats {
    pub vm_init_count: AtomicU32,
    pub total_batch_processing_time: std::time::Duration,
    pub batch_init_time: std::time::Duration,
    pub execution_time: std::time::Duration,
    pub validation_time: std::time::Duration,
    pub materialization_task_spawn_time: std::time::Duration,
    pub successful_vm_executions: AtomicU32,
    pub discarded_vm_executions: AtomicU32,
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
        txn: T,
        txn_idx: TxnIndex,
        executor_task: &E,
        state_view: &impl TStateView<Key = T::Key>,
        aggregators_snapshot: &impl WritableStateView<Key = T::Key, Value = T::Value>,
        write_reservations: &impl ReservationTable<T::Key>,
        delta_reservations: &impl ReservationTable<T::Key>,
        sequence_numbers: &DashMap<AccountAddress, u64>,
    ) -> Result<TxnExecutionInfo<E>, Error<E::Error>> {
        // TODO: eventually, we may need to add extra logic to preserve source order.

        if let Some((account, sn)) = txn.sender_and_sequence_number() {
            if let Some(last_sn) = sequence_numbers.get(&account) {
                if *last_sn + 1 != sn {
                    return Ok(TxnExecutionInfo {
                        transaction: txn,
                        read_set: vec![],
                        output: None,
                        abort: true,
                        vm_execution: false,
                        txn_idx,
                        _phantom: PhantomData,
                    });
                }
            }
        }

        let read_set_capturing_view = ReadSetCapturingStateView::with_capacity(state_view, 10);

        let status = executor_task.execute_transaction(
            &read_set_capturing_view,
            &txn,
            txn_idx,
            false
        );

        let (output, abort) = match status {
            ExecutionStatus::Success(output) => (Some(output), false),
            ExecutionStatus::Abort(_) => (None, true),
            // TODO: handle skip_rest properly
            ExecutionStatus::SkipRest(output) => (Some(output), false),
        };

        let txn_execution_info = TxnExecutionInfo {
            transaction: txn,
            read_set: read_set_capturing_view.take_read_set(),
            output,
            abort,
            vm_execution: true,
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
        transactions: impl IntoParallelIterator<Item = (usize, T)>,
        worker_states: &ThreadLocal<E>,
        executor_arguments: &E::Argument,
        vm_init_count: &AtomicU32,
        state_view: &(impl TStateView<Key = T::Key> + Sync),
        aggregators_snapshot: &(impl WritableStateView<Key = T::Key, Value = T::Value> + Sync),
        write_reservations: &(impl ReservationTable<T::Key> + Sync),
        delta_reservations: &(impl ReservationTable<T::Key> + Sync),
        sequence_numbers: &DashMap<AccountAddress, u64>,
    ) -> Result<Vec<TxnExecutionInfo<E>>, Error<E::Error>> {
        transactions
            .into_par_iter()
            .map_init(
                || worker_states.get_or(|| {
                    vm_init_count.fetch_add(1, SeqCst);
                    E::init(*executor_arguments)
                }),
                |&mut executor_task, (txn_idx, txn)| {
                    Self::execute_transaction(
                        txn,
                        txn_idx as TxnIndex,
                        executor_task,
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
                    return false;
                }
            }

            // A transaction cannot be committed if it reads an aggregator that was updated
            // by a smaller-id transaction in the same batch.
            if let Some(reservation) = delta_reservations.get_reservation(k) {
                if reservation < txn_idx {
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
        execution_results: Vec<TxnExecutionInfo<E>>,
        write_reservations: &(impl ReservationTable<T::Key> + Sync),
        delta_reservations: &(impl ReservationTable<T::Key> + Sync),
        sequence_numbers: &DashMap<AccountAddress, u64>,
        output_view: &(impl WritableStateView<Key = T::Key, Value = T::Value> + Sync),
        discarded_txns: &mut Vec<TransactionWithHints<T>>,
        fast_path_stats: &FastPathStats,
    ) -> Result<Vec<E::Output>, Error<E::Error>> {
        let error: AtomicCell<Option<Error<E::Error>>> = AtomicCell::new(None);

        let mut selected_txn_outputs = Vec::with_capacity(execution_results.len());

        (selected_txn_outputs.by_ref(), discarded_txns.by_ref()).par_extend(
            execution_results.into_par_iter().map(|exec_info| {
                let select = Self::validate_transaction(
                    &exec_info,
                    write_reservations,
                    delta_reservations,
                );

                if select {
                    fast_path_stats.successful_vm_executions.fetch_add(1, SeqCst, );

                    if let Some((account, sn)) = exec_info.transaction.sender_and_sequence_number() {
                        sequence_numbers.insert(account, sn);
                    }

                    let output = exec_info.output.unwrap();
                    let err = Self::apply_transaction_output(&output, output_view);

                    if let Err(e) = err {
                        error.store(Some(e));
                    }
                    Left(output)
                } else {
                    if exec_info.vm_execution {
                        fast_path_stats.discarded_vm_executions.fetch_add(1, SeqCst);
                    }

                    let write_set = exec_info.write_set().map(Iterator::collect).unwrap_or(vec![]);
                    let delta_set = exec_info.delta_set().map(Iterator::collect).unwrap_or(vec![]);
                    let TxnExecutionInfo{ transaction, read_set, .. } = exec_info;

                    Right(TransactionWithHints { transaction, read_set, write_set, delta_set })
                }
            }),
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

    fn process_batch(
        &self,
        transactions: impl IntoParallelIterator<Item = (usize, T)>,
        worker_states: &ThreadLocal<E>,
        executor_arguments: &E::Argument,
        writable_view: &(impl WritableStateView<Key = T::Key, Value = T::Value> + Sync),
        sequence_numbers: &DashMap<AccountAddress, u64>,
        discarded_txns: &mut Vec<TransactionWithHints<T>>,
        fast_path_stats: &mut FastPathStats,
    ) -> Result<oneshot::Receiver<Vec<E::Output>>, Error<E::Error>> {
        let start = std::time::Instant::now();
        let batch_size = self.batch_size;

        let write_reservations = DashMapReservationTable::with_capacity(4 * batch_size);
        // Aggregators are often highly contended. Using OptimisticDashMapReservationTable allows
        // avoiding having all transactions compete for the same write lock.
        let delta_reservations =
            OptimisticDashMapReservationTable::with_capacity(2 * batch_size);

        let aggregators_snapshot = DashMapStateView::new(EmptyStateView::new());

        let after_init = std::time::Instant::now();

        let execution_results = Self::execute_transactions_in_parallel(
            transactions,
            &worker_states,
            executor_arguments,
            &fast_path_stats.vm_init_count,
            &writable_view.as_state_view(),
            &aggregators_snapshot,
            &write_reservations,
            &delta_reservations,
            &sequence_numbers,
        )?;

        let after_execution = std::time::Instant::now();

        let selected_outputs = Self::validate_and_apply_transactions_in_parallel(
            execution_results,
            &write_reservations,
            &delta_reservations,
            &sequence_numbers,
            writable_view,
            discarded_txns,
            &fast_path_stats,
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
        let start = std::time::Instant::now();

        let block_size = signature_verified_block.len();
        let batch_count = (block_size + self.batch_size - 1) / self.batch_size;

        // FIXME: this is a dirty fix.
        let last_txn = signature_verified_block.pop().unwrap();

        let state = DashMapStateView::with_capacity(base_view, block_size);

        // `+ 1` is for outputs yielded by the fallback.
        let mut committed_outputs_receivers = Vec::with_capacity(batch_count + 1);

        let mut discarded_txns = Vec::<TransactionWithHints<T>>::with_capacity(block_size);
        let sequence_numbers = DashMap::with_capacity(block_size);

        let worker_states = ThreadLocal::new();

        let after_init = std::time::Instant::now();

        let mut fast_path_stats = FastPathStats::default();

        let mut transactions = signature_verified_block.into_iter().enumerate();
        while transactions.len() != 0 {
            // FIXME: this collect shouldn't be necessary.
            let batch_transactions = transactions.by_ref().take(self.batch_size).collect_vec();

            let committed_output_receiver = self.process_batch(
                batch_transactions,
                &worker_states,
                &executor_arguments,
                &state,
                &sequence_numbers,
                &mut discarded_txns,
                &mut fast_path_stats,
            )?;

            committed_outputs_receivers.push(committed_output_receiver);
        }

        discarded_txns.push(TransactionWithHints {
            transaction: last_txn,
            read_set: vec![],
            write_set: vec![],
            delta_set: vec![],
        });

        let after_fast_path = std::time::Instant::now();

        let fallback_count = discarded_txns.len();

        // Execute the discarded transactions with the fallback executor.
        let fallback_outputs = (self.fallback.lock())
            .execute_block_hinted(executor_arguments, discarded_txns, &state)?;

        let after_fallback = std::time::Instant::now();

        // Wait for all materialization tasks to finish.
        let mut committed_outputs: Vec<Vec<E::Output>> = committed_outputs_receivers
            .into_iter()
            .map(|mut receiver| loop {
                if let Some(data) = receiver.try_recv().unwrap() {
                    return data;
                }
            })
            .collect();

        let after_wait = std::time::Instant::now();
        committed_outputs.push(fallback_outputs);

        let final_output: Vec<_> = committed_outputs.into_par_iter().flatten().collect();
        let after_final_output = std::time::Instant::now();

        println!("Fast path executor stats:");
        println!("\tFallback count: {} / {}", fallback_count, block_size);
        println!("\tNumber of unique senders: {}", sequence_numbers.len());
        println!("\tTime breakdown:");
        println!("\t\tTotal: {:?}", after_final_output - start);
        println!("\t\tInit: {:?}", after_init - start);
        println!("\t\tFast path: {:?}", after_fast_path - after_init);
        println!("\t\tDetails:");
        println!("\t\t\tBatch init: {:?}", fast_path_stats.batch_init_time);
        println!("\t\t\tExecution: {:?}", fast_path_stats.execution_time);
        println!("\t\t\tValidation: {:?}", fast_path_stats.validation_time);
        println!("\t\t\tMaterialization task spawn: {:?}", fast_path_stats.materialization_task_spawn_time);
        println!("\t\t\tVM init count: {}", fast_path_stats.vm_init_count.load(SeqCst));
        println!("\t\t\tSuccessful VM executions: {:?}", fast_path_stats.successful_vm_executions.load(SeqCst));
        println!("\t\t\tDiscarded VM executions: {:?}", fast_path_stats.discarded_vm_executions.load(SeqCst));
        println!("\t\t\tMeasurement error: {:?}", (after_fast_path - after_init) - fast_path_stats.total_batch_processing_time);
        println!("\t\tFallback: {:?}", after_fallback - after_fast_path);
        println!("\t\tWait: {:?}", after_wait - after_fallback);
        println!("\t\tFinal output reconstruction: {:?}", after_final_output - after_wait);

        assert_eq!(final_output.len(), block_size);
        Ok(final_output)
    }
}

struct TxnExecutionInfo<E: ExecutorTask> {
    transaction: E::Txn,
    read_set: Vec<<E::Txn as Transaction>::Key>,
    output: Option<E::Output>,
    abort: bool,
    vm_execution: bool,
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
