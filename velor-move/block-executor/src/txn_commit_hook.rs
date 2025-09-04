// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::task::TransactionOutput;
use velor_mvhashmap::types::TxnIndex;

/// An interface for listening to transaction commit events. The listener is called only once
/// for each transaction commit.
pub trait TransactionCommitHook: Send + Sync {
    type Output;

    fn on_transaction_committed(&self, txn_idx: TxnIndex, output: &Self::Output);

    fn on_execution_aborted(&self, txn_idx: TxnIndex);
}

pub struct NoOpTransactionCommitHook<T, E> {
    phantom: std::marker::PhantomData<(T, E)>,
}

impl<T: TransactionOutput, E: Sync + Send> Default for NoOpTransactionCommitHook<T, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: TransactionOutput, E: Sync + Send> NoOpTransactionCommitHook<T, E> {
    pub fn new() -> Self {
        Self {
            phantom: std::marker::PhantomData,
        }
    }
}

impl<T: TransactionOutput, E: Sync + Send> TransactionCommitHook
    for NoOpTransactionCommitHook<T, E>
{
    type Output = T;

    fn on_transaction_committed(&self, _txn_idx: TxnIndex, _output: &Self::Output) {
        // no-op
    }

    fn on_execution_aborted(&self, _txn_idx: TxnIndex) {
        // no-op
    }
}
