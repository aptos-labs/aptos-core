// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//use crate::task::TransactionOutput;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::transaction::TransactionOutput;
use once_cell::sync::OnceCell;

/// An interface for listening to transaction commit events. The listener is called only once
/// for each transaction commit.
pub trait TransactionCommitHook: Send + Sync {
    fn on_transaction_committed(&self, txn_idx: TxnIndex, output: &OnceCell<TransactionOutput>);

    fn on_execution_aborted(&self, txn_idx: TxnIndex);
}

pub struct NoOpTransactionCommitHook<E> {
    phantom: std::marker::PhantomData<E>,
}

impl<E: Sync + Send> Default for NoOpTransactionCommitHook<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Sync + Send> NoOpTransactionCommitHook<E> {
    pub fn new() -> Self {
        Self {
            phantom: std::marker::PhantomData,
        }
    }
}

impl<E: Sync + Send> TransactionCommitHook for NoOpTransactionCommitHook<E> {
    fn on_transaction_committed(&self, _txn_idx: TxnIndex, _output: &OnceCell<TransactionOutput>) {
        // no-op
    }

    fn on_execution_aborted(&self, _txn_idx: TxnIndex) {
        // no-op
    }
}
