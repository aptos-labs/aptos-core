// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use aptos_types::transaction::BlockExecutableTransaction as Transaction;

pub trait TxnProvider<T: Transaction> {
    fn get_txn(&self, idx: usize) -> Arc<T>;

    fn num_txns(&self) -> usize;

    fn iter(&self) -> Box<dyn Iterator<Item = Arc<T>> + '_>;
}

/*pub trait TxnProviderIterator<T: Transaction> {

}*/