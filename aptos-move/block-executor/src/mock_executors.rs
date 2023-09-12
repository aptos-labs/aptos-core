// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::PARALLEL_EXECUTION_SECONDS,
    errors::{Error, Result},
    executor_traits::{BlockExecutor, BlockExecutorBase},
    task::{ExecutionStatus, ExecutorTask, Transaction},
};
use aptos_mvhashmap::types::TxnIndex;
use aptos_state_view::TStateView;
use aptos_types::executable::Executable;
use rayon::{prelude::*, ThreadPool};
use std::{marker::PhantomData, sync::Arc};
use thread_local::ThreadLocal;

/// A `BlockExecutor` that simply runs all the transactions in the block once, all in parallel,
/// using the `par_iter` API of `rayon`.
/// Provides a kind of a lower bound on the execution time of any `BlockExecutor`.
/// The execution result is *not* serializable.
/// This `BlockExecutor` is meant to be used only for performance testing.
pub struct RunAllOnceInParallel<T, E> {
    executor_thread_pool: Arc<ThreadPool>,

    phantom: PhantomData<(T, E)>,
}

impl<T, E> RunAllOnceInParallel<T, E>
where
    T: Transaction,
    E: ExecutorTask<Txn = T> + Send,
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
        state_view: &(impl TStateView<Key = T::Key> + Sync),
    ) -> Result<Vec<E::Output>, E::Error>
    where
        Idx: IntoParallelIterator<Item = TxnIndex>,
        Idx::Iter: IndexedParallelIterator,
    {
        transactions
            .par_iter()
            .zip(transactions_indices)
            .map_init(
                || executor_tasks.get_or(|| E::init(*executor_arguments)),
                |executor_task, (txn, txn_idx)| {
                    let execute_result =
                        executor_task.execute_transaction(state_view, txn, txn_idx, true);
                    match execute_result {
                        ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                            Ok(output)
                        },
                        ExecutionStatus::Abort(err) => Err(Error::UserError(err)),
                    }
                },
            )
            .collect()
    }
}

impl<T, E> BlockExecutorBase for RunAllOnceInParallel<T, E>
where
    T: Transaction,
    E: ExecutorTask<Txn = T> + Send,
{
    type Error = Error<E::Error>;
    type ExecutorTask = E;
    type Txn = T;
}

impl<T, E> BlockExecutor for RunAllOnceInParallel<T, E>
where
    T: Transaction,
    E: ExecutorTask<Txn = T> + Send,
{
    fn execute_block<S: TStateView<Key = T::Key> + Sync>(
        &self,
        executor_arguments: E::Argument,
        signature_verified_block: Vec<T>,
        base_view: &S,
    ) -> Result<Vec<E::Output>, E::Error> {
        let _timer = PARALLEL_EXECUTION_SECONDS.start_timer();

        self.execute_transactions(
            &signature_verified_block,
            0..signature_verified_block.len() as TxnIndex,
            &ThreadLocal::with_capacity(self.executor_thread_pool.current_num_threads()),
            &executor_arguments,
            base_view,
        )
    }
}
