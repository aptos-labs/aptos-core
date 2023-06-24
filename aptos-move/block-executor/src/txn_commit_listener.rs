// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{task::TransactionOutput, txn_last_input_output::TxnOutput};
use aptos_mvhashmap::types::TxnIndex;
use std::fmt::Debug;

pub trait TransactionCommitListener: Send + Sync {
    type TxnOutput;

    fn on_transaction_committed(&self, txn_idx: TxnIndex, txn_output: &Self::TxnOutput);
}

pub struct NoOpTransactionCommitListener<T, E> {
    phantom: std::marker::PhantomData<(T, E)>,
}

impl<T: TransactionOutput, E: Debug + Sync + Send> Default for NoOpTransactionCommitListener<T, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: TransactionOutput, E: Debug + Sync + Send> NoOpTransactionCommitListener<T, E> {
    pub fn new() -> Self {
        Self {
            phantom: std::marker::PhantomData,
        }
    }
}

impl<T: TransactionOutput, E: Debug + Sync + Send> TransactionCommitListener
    for NoOpTransactionCommitListener<T, E>
{
    type TxnOutput = TxnOutput<T, E>;

    fn on_transaction_committed(&self, _txn_idx: TxnIndex, _txn_writes: &Self::TxnOutput) {
        // no-op
    }
}
