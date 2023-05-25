// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{TXN_DEDUP_FILTERED, TXN_DEDUP_SECONDS},
    transaction_deduper::TransactionDeduper,
};
use aptos_types::transaction::SignedTransaction;
use std::collections::HashSet;

pub struct TxnAndAuthenticatorDeduper {}

impl TransactionDeduper for TxnAndAuthenticatorDeduper {
    fn dedup(&self, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        let _timer = TXN_DEDUP_SECONDS.start_timer();
        let txns_len = txns.len();
        let mut seen = HashSet::new();
        let deduped_txns: Vec<_> = txns
            .iter()
            .filter(|txn| seen.insert((txn.raw_transaction_ref(), txn.authenticator_ref())))
            .cloned()
            .collect();
        TXN_DEDUP_FILTERED.observe((txns_len - deduped_txns.len()) as f64);
        deduped_txns
    }
}

impl TxnAndAuthenticatorDeduper {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_dummy() {}
}
