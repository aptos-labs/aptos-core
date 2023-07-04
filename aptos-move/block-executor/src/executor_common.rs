// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_state_view::TStateView;
use aptos_types::executable::Executable;
use crate::task::{ExecutorTask, Transaction};

/// A common trait for all implementations of the execution layer.
pub trait BlockExecutor {
    type Transaction: Transaction;
    type ExecutorTask: ExecutorTask<Txn = Self::Transaction>;
    type StateView: TStateView<Key = <Self::Transaction as Transaction>::Key> + Sync;
    type Executable: Executable + 'static;
    type Error;

    /// Executes all transactions in a block and returns the outputs.
    /// The output must be *serializable*, meaning that it must be equivalent to
    /// executing the transactions in *some* total order (which may be different from the order
    /// of the transactions in the input).
    fn execute_block(
        &self,
        executor_arguments: <Self::ExecutorTask as ExecutorTask>::Argument,
        signature_verified_block: Vec<Self::Transaction>,
        base_view: &Self::StateView,
    ) -> Result<Vec<<Self::ExecutorTask as ExecutorTask>::Output>, Self::Error>;
}
