// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_mvhashmap::types::TxnIndex;

/// An interface for listening to transaction commit events. The listener is called only once
/// for each transaction commit.
pub trait TransactionCommitHook<O>: Send + Sync {
    fn on_transaction_committed(&self, txn_idx: TxnIndex, output: &O);
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

impl<O, E: Sync + Send> TransactionCommitHook<O> for NoOpTransactionCommitHook<E> {
    fn on_transaction_committed(&self, _txn_idx: TxnIndex, _output: &O) {
        // no-op
    }
}
