// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::task::Transaction;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::write_set::WriteOp;

pub trait TransactionCommitListener: Send + Sync {
    type TransactionWrites;

    fn on_transaction_committed(&self, txn_idx: TxnIndex, txn_writes: &Self::TransactionWrites);
}

pub struct NoOpTransactionCommitListener<T> {
    phantom: std::marker::PhantomData<T>,
}

impl<T> Default for NoOpTransactionCommitListener<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> NoOpTransactionCommitListener<T> {
    pub fn new() -> Self {
        Self {
            phantom: std::marker::PhantomData,
        }
    }
}

impl<T> TransactionCommitListener for NoOpTransactionCommitListener<T>
where
    T: Transaction,
{
    type TransactionWrites = Vec<(T::Key, WriteOp)>;

    fn on_transaction_committed(&self, _txn_idx: TxnIndex, _txn_writes: &Self::TransactionWrites) {
        // no-op
    }
}
