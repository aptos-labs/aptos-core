// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod blocking_txn_provider;
pub mod default;

use crate::transaction::BlockExecutableTransaction as Transaction;

pub type TxnIndex = u32;

pub trait TxnProvider<T: Transaction>: Send + Sync {
    /// Get total number of transactions
    fn num_txns(&self) -> usize;

    /// Get a reference of the txn object by its index.
    fn get_txn(&self, idx: TxnIndex) -> &T;

    fn to_vec(&self) -> Vec<T>;
}
