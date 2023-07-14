// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{marker::PhantomData, sync::Arc};
use std::cell::RefCell;

use futures::channel::oneshot;
use futures::SinkExt;
use rayon::prelude::*;
use rayon::ThreadPool;
use thread_local::ThreadLocal;

use aptos_aggregator::delta_change_set::serialize;
use aptos_logger::Value;
use aptos_mvhashmap::types::TxnIndex;
use aptos_state_view::TStateView;
use aptos_types::executable::Executable;
use aptos_types::write_set::WriteOp;
use anyhow::Result;
use rayon::iter::Either::{Left, Right};

use crate::{
    counters::{
        PARALLEL_EXECUTION_SECONDS, RAYON_EXECUTION_SECONDS, TASK_EXECUTE_SECONDS,
        TASK_VALIDATE_SECONDS, VM_INIT_SECONDS, WORK_WITH_TASK_SECONDS,
    },
    errors::*,
    executor_common::BlockExecutor,
    scheduler::{DependencyStatus, ExecutionTaskType, Scheduler, SchedulerTask, Wave},
    task::{ExecutionStatus, ExecutorTask, Transaction, TransactionOutput},
    txn_last_input_output::TxnLastInputOutput,
    view::{LatestView, MVHashMapView},
};
use crate::fast_path_executor::{
    reservation_table::{DashMapReservationTable, ReservationTable},
    view::ReadSetCapturingStateView,
};
use crate::fast_path_executor::reservation_table::OptimisticDashMapReservationTable;
use crate::fast_path_executor::view::{DashMapStateView, EmptyStateView, WritableStateView};

// Fake BlockExecutor to use a placeholder type parameter.
pub struct NoFallback<T, E, S, X> {
    phantom: PhantomData<(T, E, S, X)>,
}

impl<T, E, S, X> BlockExecutor for NoFallback<T, E, S, X>
where
    T: Transaction,
    E: ExecutorTask<Txn = T>,
    S: TStateView<Key = T::Key> + Sync,
    X: Executable + 'static,
{
    type Transaction = T;
    type ExecutorTask = E;
    type StateView = S;
    type Executable = X;
    type Error = anyhow::Error;

    fn execute_block(
        &self,
        _executor_arguments: E::Argument,
        _signature_verified_block: Vec<T>,
        _base_view: &S,
    ) -> Result<Vec<E::Output>> {
        panic!("This function should never be called.");
    }
}

// TODO: there should be some default value for FB for FastPathBlockExecutor::without_fallback to work as expected.
pub struct FastPathBlockExecutor<T: Transaction, E, S, X, FB = NoFallback<T, E, S, X>> {
    executor_thread_pool: Arc<ThreadPool>,
    maybe_block_gas_limit: Option<u64>,
    maybe_fallback: Option<FB>,

    phantom: PhantomData<(T, E, S, X)>,
}

impl<T, E, S, X, FB> FastPathBlockExecutor<T, E, S, X, FB>
where
    T: Transaction,
    E: ExecutorTask<Txn = T> + Send,
    E::Error: std::error::Error,
    S: TStateView<Key = T::Key> + Sync,
    X: Executable + 'static,
    FB: BlockExecutor<Transaction = T, ExecutorTask = E, StateView = S, Executable = X>,
{
    pub fn new(
        executor_thread_pool: Arc<ThreadPool>,
        maybe_block_gas_limit: Option<u64>,
        maybe_fallback: Option<FB>,
    ) -> Self {
        Self {
            executor_thread_pool,
            maybe_block_gas_limit,
            maybe_fallback,
            phantom: PhantomData,
        }
    }

    pub fn without_fallback(
        executor_thread_pool: Arc<ThreadPool>,
        maybe_block_gas_limit: Option<u64>,
    ) -> Self {
        Self::new(executor_thread_pool, maybe_block_gas_limit, None)
    }

    pub fn with_fallback(
        executor_thread_pool: Arc<ThreadPool>,
        maybe_block_gas_limit: Option<u64>,
        fallback: FB,
    ) -> Self {
        Self::new(executor_thread_pool, maybe_block_gas_limit, Some(fallback))
    }

    fn execute_transaction(
        txn: &T,
        txn_idx: TxnIndex,
        executor_task: &E,
        state_view: &impl TStateView<Key = T::Key>,
        accumulators_snapshot: &impl WritableStateView<Key = T::Key, Value = T::Value>,
        write_reservations: &impl ReservationTable<T::Key>,
        delta_reservations: &impl ReservationTable<T::Key>,
    ) -> Result<TxnExecutionInfo<E>> {
        // TODO: eventually, we may need to add extra logic to preserve source order.

        let read_set_capturing_view = ReadSetCapturingStateView::with_capacity(state_view, 10);

        let execute_result = executor_task.execute_transaction(
            &read_set_capturing_view,
            txn,
            txn_idx,
            false
        );

        let (skip_rest, output) = match execute_result {
            ExecutionStatus::Success(output) => (false, output),
            ExecutionStatus::SkipRest(output) => (true, output),
            ExecutionStatus::Abort(err) => return Err(err.into()),
        };


        for (k, _) in output.get_writes().into_iter() {
            write_reservations.make_reservation(k, txn_idx);
        }

        for (k, _) in output.get_deltas() {
            // As accumulators can be highly contended, first check if the snapshot
            // of this accumulator has already been made and only proceed to making
            // the snapshot otherwise.
            if let None = accumulators_snapshot.get_state_value_u128(&k)? {
                let val = state_view.get_state_value_u128(&k)?
                    .ok_or(anyhow::Error::msg("Writing to a non-existent accumulator"))?;
                accumulators_snapshot.write_u128(k.clone(), val);
            }

            delta_reservations.make_reservation(k, txn_idx);
        }

        Ok(TxnExecutionInfo {
            read_set: read_set_capturing_view.clone_read_set(),
            output,
            skip_rest,
            phantom: PhantomData,
        })
    }

    fn execute_transactions_in_parallel<'data, Txns, Idx>(
        transactions: Txns,
        indices: Idx,
        worker_states: &ThreadLocal<E>,
        executor_arguments: &E::Argument,
        state_view: &(impl TStateView<Key = T::Key> + Sync),
        accumulators_snapshot: &(impl WritableStateView<Key = T::Key, Value = T::Value> + Sync),
        write_reservations: &(impl ReservationTable<T::Key> + Sync),
        delta_reservations: &(impl ReservationTable<T::Key> + Sync),
    ) -> Result<Vec<TxnExecutionInfo<E>>>
    where
        Txns: IntoParallelIterator<Item = &'data T>,
        Txns::Iter: IndexedParallelIterator,
        Idx: IntoParallelIterator<Item = TxnIndex>,
        Idx::Iter: IndexedParallelIterator,
    {
        transactions
            .into_par_iter()
            .zip(indices)
            .map_init(
                || worker_states.get_or(|| E::init(*executor_arguments)),
                |&mut executor_task, (txn, txn_idx)| {
                    Self::execute_transaction(
                        txn,
                        txn_idx,
                        executor_task,
                        state_view,
                        accumulators_snapshot,
                        write_reservations,
                        delta_reservations,
                    )
                },
            )
            .collect()
    }

    fn validate_transaction<'data>(
        txn_reads: impl IntoIterator<Item = &'data T::Key>,
        _txn_output: &E::Output, // This will be used in later, more advanced prototypes.
        txn_idx: TxnIndex,
        write_reservations: &(impl ReservationTable<T::Key> + Sync),
        delta_reservations: &(impl ReservationTable<T::Key> + Sync),
    ) -> bool {
        for k in txn_reads.into_iter() {
            let k: &T::Key = k; // help the IDE figure out the type of `k`

            // A transaction cannot be committed on the fast path if a smaller-id transaction
            // writes to a location this transaction reads.
            if let Some(reservation) = write_reservations.get_reservation(k) {
                if reservation < txn_idx {
                    return false;
                }
            }

            if let Some(reservation) = delta_reservations.get_reservation(k) {
                if reservation < txn_idx {
                    return false;
                }
            }
        }

        // We do not need to track write-write conflicts as each transaction always
        // reads a key before writing to it.

        // TODO: later we will need to add accumulator overflow detection
        true
    }

    fn apply_transaction_output(
        txn_output: &E::Output,
        state: &(impl WritableStateView<Key = T::Key, Value = T::Value> + Sync),
    ) -> Result<()> {
        for (k, v) in txn_output.get_writes() {
            state.write_value(k, v);
        }

        for (k, delta_op) in txn_output.get_deltas() {
            state.apply_delta(&k, &delta_op)?;
        }

        Ok(())
    }

    fn validate_and_apply_transactions_in_parallel(
        execution_results: Vec<TxnExecutionInfo<E>>,
        write_reservations: &(impl ReservationTable<T::Key> + Sync),
        delta_reservations: &(impl ReservationTable<T::Key> + Sync),
        writable_view: &(impl WritableStateView<Key = T::Key, Value = T::Value> + Sync),
    ) -> Result<(Vec<TxnExecutionInfo<E>>, Vec<TxnExecutionInfo<E>>)> {
        todo!()
        // let (committed_info, other_info): (Result<Vec<_>>, Result<Vec<_>>) =
        //     execution_results
        //         .into_par_iter()
        //         .partition_map(|execution_result| {
        //             let commit = Self::validate_transaction(
        //                 txn_reads,
        //                 txn_output,
        //                 txn_idx,
        //                 write_reservations,
        //                 delta_reservations,
        //             );
        //
        //             if commit {
        //                 Self::apply_transaction_output(txn_output, writable_view)?;
        //                 Left(Ok(txn_idx))
        //             } else {
        //                 Right(Ok(txn_idx))
        //             }
        //         });
        //
        // Ok((committed_info?, other_info?))
    }

    // fn materialize_deltas<'data, Outputs, View>(
    //     outputs_in_serialization_order: Outputs,
    //     base_view: &View,
    // ) -> Result<()>
    // where
    //     Outputs: IntoIterator<Item = &'data E::Output>,
    //     View: TStateView<Key = T::Key>,
    // {
    //     let updated_view = DashMapStateView::<_, T::Value, _>::new(base_view);
    //
    //     for output in outputs_in_serialization_order {
    //         let deltas = output.get_deltas();
    //         let mut delta_writes = Vec::with_capacity(deltas.len());
    //
    //         for (k, delta) in deltas {
    //             let new_value = updated_view.apply_delta(&k, &delta)?;
    //             delta_writes.push((k, WriteOp::Modification(serialize(&new_value))))
    //         }
    //
    //         output.incorporate_delta_writes(delta_writes);
    //     }
    //
    //     Ok(())
    // }

    fn materialize_deltas<'data>(
        outputs_in_serialization_order: impl IntoIterator<Item = &'data E::Output>,
        base_view: impl TStateView<Key = T::Key>,
    ) -> Result<()> {
        let updated_view = DashMapStateView::<_, T::Value, _>::new(base_view);

        for output in outputs_in_serialization_order {
            let deltas = output.get_deltas();
            let mut delta_writes = Vec::with_capacity(deltas.len());

            for (k, delta) in deltas {
                let new_value = updated_view.apply_delta(&k, &delta)?;
                delta_writes.push((k, WriteOp::Modification(serialize(&new_value))))
            }

            output.incorporate_delta_writes(delta_writes);
        }

        Ok(())
    }

    fn process_batch(
        &self,
        transactions: &[T],
        executor_arguments: &E::Argument,
        writable_view: &(impl WritableStateView<Key = T::Key, Value = T::Value> + Sync),
        committed_outputs: &mut Vec<E::Output>,
        discarded_txns_info: &mut Vec<TxnExecutionInfo<E>>
    ) -> Result<BatchExecutionOutput<E::Output>> {
        let transaction_count = transactions.len();

        let write_reservations = DashMapReservationTable::with_capacity(transaction_count * 2);
        // Accumulators are often highly contended. Using OptimisticDashMapReservationTable allows
        // avoiding having all transactions compete for the same write lock.
        let delta_reservations = OptimisticDashMapReservationTable::with_capacity(transaction_count);

        let worker_states = ThreadLocal::new();

        let accumulators_snapshot = DashMapStateView::new(EmptyStateView::new());

        let execution_results = Self::execute_transactions_in_parallel(
            transactions,
            0..transaction_count as TxnIndex,
            &worker_states,
            executor_arguments,
            &writable_view.as_state_view(),
            &accumulators_snapshot,
            &write_reservations,
            &delta_reservations,
        )?;

        let (committed_results, other_results) = Self::validate_and_apply_transactions_in_parallel(
            execution_results,
            &write_reservations,
            &delta_reservations,
            writable_view,
        )?;

        // let (done_tx, done_rx) = oneshot::channel();

        // Materialize deltas asynchronously, off the critical path
        rayon::spawn(move || {
            // let res = Self::materialize_deltas(
            //     committed_results.iter().map(|info| &info.output),
            //     accumulators_snapshot
            // );
            todo!()
            // done_tx.send(committed_results.into_iter()).unwrap();
        });

        // Ok(BatchExecutionOutput{
        //     committed_outputs:
        // })
        todo!()
    }

    /// This function should be executed in the context of the thread pool via
    /// `self.executor_thread_pool.install`.
    fn execute_block_impl(
        &self,
        executor_arguments: E::Argument,
        signature_verified_block: Vec<T>,
        base_view: &S,
    ) -> Result<Vec<E::Output>> {
        let block_size = signature_verified_block.len();
        let batch_size = (block_size / 10).min(MAX_BATCH_SIZE).max(MIN_BATCH_SIZE);
        let batch_count = (block_size + batch_size - 1) / batch_size;

        let state = DashMapStateView::with_capacity(base_view, block_size);

        let mut committed_outputs_in_serialization_order = vec![];

        let mut committed_outputs = Vec::<E::Output>::with_capacity(block_size);
        let mut discarded_txns_info = Vec::<TxnExecutionInfo<_>>::with_capacity(block_size);

        for batch_idx in 0..batch_count {
            let batch_begin = batch_idx * batch_size;
            let batch_end = ((batch_idx + 1) * batch_size).min(block_size);
            let batch_transactions = &signature_verified_block[batch_begin..batch_end];
            let batch_res = self.process_batch(
                batch_transactions,
                &executor_arguments,
                &state,
                &mut committed_outputs,
                &mut discarded_txns_info,
            )?;

            committed_outputs_in_serialization_order.par_extend(
                batch_res.committed_outputs.into_par_iter().map(|(out, _id)| out));
        }

        todo!()
    }
}

struct TxnExecutionInfo<E: ExecutorTask> {
    read_set: Vec<<E::Txn as Transaction>::Key>,
    output: E::Output,
    skip_rest: bool,

    phantom: PhantomData<E>,
}

struct BatchExecutionOutput<O> {
    committed_outputs: Vec<(O, TxnIndex)>,
    discarded_outputs: Vec<(O, TxnIndex)>,
    materialization_complete: oneshot::Receiver<Result<()>>,
}

const MAX_BATCH_SIZE: usize = 2000;
const MIN_BATCH_SIZE: usize = 200;

impl<T, E, S, X, FB> BlockExecutor for FastPathBlockExecutor<T, E, S, X, FB>
where
    T: Transaction,
    E: ExecutorTask<Txn = T> + Send,
    E::Error: std::error::Error,
    S: TStateView<Key = T::Key> + Sync,
    X: Executable + 'static,
    FB: BlockExecutor<Transaction = T, ExecutorTask = E, StateView = S, Executable = X> + Sync,
{
    type Transaction = T;
    type ExecutorTask = E;
    type StateView = S;
    type Executable = X;
    type Error = anyhow::Error;

    fn execute_block(
        &self,
        executor_arguments: E::Argument,
        signature_verified_block: Vec<T>,
        base_view: &S,
    ) -> Result<Vec<E::Output>> {
        self.executor_thread_pool.install(|| {
            self.execute_block_impl(executor_arguments, signature_verified_block, base_view)
        })
    }
}
