// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod blocking_txns_provider;
pub mod default;

use aptos_mvhashmap::types::TxnIndex;
use aptos_types::transaction::{AuxiliaryInfoTrait, BlockExecutableTransaction as Transaction};

pub trait TxnProvider<T: Transaction, A: AuxiliaryInfoTrait> {
    /// Get total number of transactions
    fn num_txns(&self) -> usize;

    /// Get a reference of the txn object by its index.
    fn get_txn(&self, idx: TxnIndex) -> &T;

    fn get_auxiliary_info(&self, idx: TxnIndex) -> A;
}
