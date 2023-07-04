// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{marker::PhantomData, sync::Arc};

use futures::channel::oneshot;
use futures::SinkExt;
use rayon::prelude::*;
use rayon::ThreadPool;
use thread_local::ThreadLocal;

use aptos_aggregator::delta_change_set::serialize;
use aptos_mvhashmap::types::TxnIndex;
use aptos_state_view::TStateView;
use aptos_types::executable::Executable;
use aptos_types::write_set::WriteOp;
use anyhow::Result;
use crossbeam::atomic::AtomicCell;

use crate::{
    executor_common::BlockExecutor,
    task::{ExecutionStatus, ExecutorTask, Transaction, TransactionOutput},
};
use crate::fast_path_executor::{
    reservation_table::{DashMapReservationTable, ReservationTable},
    view::ReadSetCapturingStateView,
};
use crate::fast_path_executor::reservation_table::OptimisticDashMapReservationTable;
use crate::fast_path_executor::view::{DashMapStateView, EmptyStateView, WritableStateView};

pub struct FastPathBlockExecutor<T: Transaction, E, S, X, FB> {
    executor_thread_pool: Arc<ThreadPool>,
    maybe_block_gas_limit: Option<u64>,
    fallback: FB,

    phantom: PhantomData<(T, E, S, X)>,
}

impl<T, E, S, X, FB> FastPathBlockExecutor<T, E, S, X, FB>
where
    T: Transaction,
    E: ExecutorTask<Txn = T> + Send + Sync + 'static,
    E::Error: std::error::Error,
    E::Output: Send,
    S: TStateView<Key = T::Key> + Sync,
    X: Executable + 'static,
    FB: BlockExecutor<Transaction = T, ExecutorTask = E, StateView = S, Executable = X>,
{
    pub fn new(
        executor_thread_pool: Arc<ThreadPool>,
        maybe_block_gas_limit: Option<u64>,
        fallback: FB,
    ) -> Self {
        Self {
            executor_thread_pool,
            maybe_block_gas_limit,
            fallback,
            phantom: PhantomData,
        }
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
            txn_idx,
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

    fn validate_transaction(
        execution_info: &TxnExecutionInfo<E>,
        write_reservations: &(impl ReservationTable<T::Key> + Sync),
        delta_reservations: &(impl ReservationTable<T::Key> + Sync),
    ) -> bool {
        let txn_idx = execution_info.txn_idx;
        for k in execution_info.read_set.iter() {
            let k: &T::Key = k; // help the IDE figure out the type of `k`

            // A transaction cannot be committed if it reads a location that is written by
            // a smaller-id transaction in the same batch.
            if let Some(reservation) = write_reservations.get_reservation(k) {
                if reservation < txn_idx {
                    return false;
                }
            }

            // A transaction cannot be committed if it reads an accumulator that was updated
            // by a smaller-id transaction in the same batch.
            if let Some(reservation) = delta_reservations.get_reservation(k) {
                if reservation < txn_idx {
                    return false;
                }
            }
        }

        // NB: We do not need to track write-write conflicts as each transaction always
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
        let mut err: AtomicCell<Option<anyhow::Error>> = AtomicCell::new(None);

        let (committed_info, other_info): (Vec<_>, Vec<_>) =
            execution_results
                .into_par_iter()
                .partition(|execution_result| {
                    let commit = Self::validate_transaction(
                        &execution_result,
                        write_reservations,
                        delta_reservations,
                    );

                    if commit {
                        if let Err(e) = Self::apply_transaction_output(&execution_result.output, writable_view) {
                            err.store(Some(e));
                        }
                    }

                    commit
                });

        if let Some(e) = err.take() {
            return Err(e);
        }

        Ok((committed_info, other_info))
    }

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
    ) -> Result<BatchExecutionOutput<E>> {
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

        let (done_tx, done_rx) = oneshot::channel::<Vec<E::Output>>();

        // Materialize deltas asynchronously, off the critical path.
        // Take the ownership of the committed outputs for the materialization and then
        // return it via the channel.
        rayon::spawn(move || {
            Self::materialize_deltas(
                committed_results.iter().map(|info| &info.output),
                accumulators_snapshot
            ).unwrap();  // TODO: potentially handle the materialization errors

            done_tx.send(committed_results.into_iter().map(|info| info.output).collect()).unwrap();
        });

        Ok(BatchExecutionOutput{
            committed_info_receiver: done_rx,
            discarded_info: other_results,
            phantom: PhantomData,
        })
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

        let mut committed_outputs_receivers = vec![];
        // let mut discarded_txns_info = Vec::<TxnExecutionInfo<_>>::with_capacity(block_size);

        for batch_idx in 0..batch_count {
            let batch_begin = batch_idx * batch_size;
            let batch_end = ((batch_idx + 1) * batch_size).min(block_size);
            let batch_transactions = &signature_verified_block[batch_begin..batch_end];
            let batch_res = self.process_batch(
                batch_transactions,
                &executor_arguments,
                &state,
            )?;
            let BatchExecutionOutput{committed_info_receiver, discarded_info, ..} = batch_res;
            committed_outputs_receivers.push(committed_info_receiver);
        }

        // Wait for all materialization tasks to finish.
        let committed_outputs: Vec<Vec<E::Output>> = committed_outputs_receivers.into_iter().map(|mut receiver| {
            loop {
                if let Some(data) = receiver.try_recv().unwrap() {
                    return data;
                }
            }
        }).collect();

        // TODO: execute the fallback

        Ok(committed_outputs.into_par_iter().flatten().collect())
    }
}

struct TxnExecutionInfo<E: ExecutorTask> {
    read_set: Vec<<E::Txn as Transaction>::Key>,
    output: E::Output,
    skip_rest: bool,
    txn_idx: TxnIndex,

    phantom: PhantomData<E>,
}

struct BatchExecutionOutput<E: ExecutorTask> {
    committed_info_receiver: oneshot::Receiver<Vec<E::Output>>,
    discarded_info: Vec<TxnExecutionInfo<E>>,

    phantom: PhantomData<E>,
}

const MAX_BATCH_SIZE: usize = 2000;
const MIN_BATCH_SIZE: usize = 200;

impl<T, E, S, X, FB> BlockExecutor for FastPathBlockExecutor<T, E, S, X, FB>
where
    T: Transaction,
    E: ExecutorTask<Txn = T> + Send + 'static,
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
