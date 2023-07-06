// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    errors::Error,
    task::{ExecutionStatus, TransactionOutput},
};
use aptos_mvhashmap::types::TxnIndex;
use std::fmt::Debug;

/// An interface for listening to transaction commit events. The listener is called only once
/// for each transaction commit.
pub trait TransactionCommitHook: Send + Sync {
    type ExecutionStatus;

    fn on_transaction_committed(&self, txn_idx: TxnIndex, execution_status: &Self::ExecutionStatus);
}

pub struct NoOpTransactionCommitHook<T, E> {
    phantom: std::marker::PhantomData<(T, E)>,
}

impl<T: TransactionOutput, E: Debug + Sync + Send> Default for NoOpTransactionCommitHook<T, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: TransactionOutput, E: Debug + Sync + Send> NoOpTransactionCommitHook<T, E> {
    pub fn new() -> Self {
        Self {
            phantom: std::marker::PhantomData,
        }
    }
}

impl<T: TransactionOutput, E: Debug + Sync + Send> TransactionCommitHook
    for NoOpTransactionCommitHook<T, E>
{
    type ExecutionStatus = ExecutionStatus<T, Error<E>>;

    fn on_transaction_committed(
        &self,
        _txn_idx: TxnIndex,
        _execution_status: &Self::ExecutionStatus,
    ) {
        // no-op
    }
}
