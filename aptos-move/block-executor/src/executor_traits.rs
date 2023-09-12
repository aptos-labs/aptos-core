// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    hints::TransactionHints,
    task::{ExecutorTask, IntoTransaction, Transaction},
};
use aptos_state_view::TStateView;
use aptos_types::executable::Executable;

/// The base trait for all block executors.
pub trait BlockExecutorBase {
    type Txn: Transaction;
    type ExecutorTask: ExecutorTask<Txn = Self::Txn>;
    type Error;
}

/// Trait for block executors that accept transactions as is, without hints
/// about the keys they may access.
pub trait BlockExecutor: BlockExecutorBase {
    /// Executes all transactions in a block and returns the outputs.
    /// The output must be *serializable*, meaning that it must be equivalent to
    /// executing the transactions in *some* total order (which may be different from the order
    /// of the transactions in the input).
    /// Returns the transaction outputs in the serialization order.
    fn execute_block<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        executor_arguments: <Self::ExecutorTask as ExecutorTask>::Argument,
        signature_verified_block: Vec<Self::Txn>,
        base_view: &S,
    ) -> Result<Vec<<Self::ExecutorTask as ExecutorTask>::Output>, Self::Error>;
}

/// Trait for block executors that accept transactions with hints about the keys they may access.
pub trait HintedBlockExecutor<HT>: BlockExecutorBase
where
    HT: TransactionHints<Key = <Self::Txn as Transaction>::Key> + IntoTransaction<Txn = Self::Txn>,
{
    fn execute_block_hinted<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        executor_arguments: <Self::ExecutorTask as ExecutorTask>::Argument,
        hinted_transactions: Vec<HT>,
        base_view: &S,
    ) -> Result<Vec<<Self::ExecutorTask as ExecutorTask>::Output>, Self::Error>;
}
