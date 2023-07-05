// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;
use std::sync::Arc;
use rayon::ThreadPool;
use rayon::prelude::*;
use thread_local::ThreadLocal;
use aptos_mvhashmap::types::TxnIndex;
use aptos_state_view::TStateView;
use aptos_types::block_executor::partitioner::BlockExecutorTransactions;
use aptos_types::executable::Executable;
use crate::counters::PARALLEL_EXECUTION_SECONDS;
use crate::errors::{Error, Result};
use crate::executor_common::BlockExecutor;
use crate::task::{ExecutionStatus, ExecutorTask, Transaction};

/// A `BlockExecutor` that simply runs all the transactions in the block once, all in parallel,
/// using the `par_iter` API of `rayon`.
/// Provides a kind of a lower bound on the execution time of any `BlockExecutor`.
/// The execution result is *not* serializable.
/// This `BlockExecutor` is meant to be used only for performance testing.
pub struct RunAllOnceInParallel<T, E, S, X> {
    executor_thread_pool: Arc<ThreadPool>,

    phantom: PhantomData<(T, E, S, X)>,
}

impl<T, E, S, X> RunAllOnceInParallel<T, E, S, X>
where
    T: Transaction,
    E: ExecutorTask<Txn = T> + Send,
    S: TStateView<Key = T::Key> + Sync,
    X: Executable + 'static,
{
    pub fn new(executor_thread_pool: Arc<ThreadPool>) -> Self {
        Self {
            executor_thread_pool,
            phantom: PhantomData,
        }
    }

    fn execute_transactions<Idx>(
        &self,
        transactions: &[T],
        transactions_indices: Idx,
        executor_tasks: &ThreadLocal<E>,
        executor_arguments: &E::Argument,
        state_view: &S,
    ) -> Result<Vec<E::Output>, E::Error>
    where
        Idx: IntoParallelIterator<Item = TxnIndex>,
        Idx::Iter: IndexedParallelIterator,
    {
        transactions.par_iter()
            .zip(transactions_indices)
            .map_init(
                || executor_tasks.get_or(|| E::init(*executor_arguments)),
                |executor_task, (txn, txn_idx)| {
                    let execute_result = executor_task.execute_transaction(
                        state_view,
                        txn,
                        txn_idx,
                        true);
                    match execute_result {
                        ExecutionStatus::Success(output) => {
                            Ok(output)
                        },
                        ExecutionStatus::SkipRest(output) => {
                            Ok(output)
                        },
                        ExecutionStatus::Abort(err) => {
                            Err(Error::UserError(err))
                        }
                    }
                },
            )
            .collect()
    }
}

impl<T, E, S, X> BlockExecutor for RunAllOnceInParallel<T, E, S, X>
    where
        T: Transaction,
        E: ExecutorTask<Txn = T> + Send,
        S: TStateView<Key = T::Key> + Sync,
        X: Executable + 'static,
{
    type Transaction = T;
    type ExecutorTask = E;
    type StateView = S;
    type Executable = X;
    type Error = Error<E::Error>;

    fn execute_block(
        &self,
        executor_arguments: E::Argument,
        signature_verified_block: BlockExecutorTransactions<T>,
        base_view: &S,
    ) -> Result<Vec<E::Output>, E::Error> {
        let _timer = PARALLEL_EXECUTION_SECONDS.start_timer();

        let signature_verified_txns = signature_verified_block.into_txns();
        self.execute_transactions(
            &signature_verified_txns,
            0..signature_verified_txns.len() as TxnIndex,
            &ThreadLocal::with_capacity(self.executor_thread_pool.current_num_threads()),
            &executor_arguments,
            base_view,
        )
    }
}
