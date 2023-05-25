// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{TXN_DEDUP_FILTERED, TXN_DEDUP_SECONDS},
    transaction_deduper::TransactionDeduper,
};
use aptos_types::transaction::SignedTransaction;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

pub struct TxnAndAuthenticatorDeduper {}

impl TransactionDeduper for TxnAndAuthenticatorDeduper {
    fn dedup(&self, transactions: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        let _timer = TXN_DEDUP_SECONDS.start_timer();
        let mut seen = HashMap::new();
        let mut is_possible_duplicate = false;
        let mut possible_duplicates = vec![false; transactions.len()];
        for (i, txn) in transactions.iter().enumerate() {
            match seen.get(&(txn.sender(), txn.sequence_number())) {
                None => {
                    seen.insert((txn.sender(), txn.sequence_number()), i);
                },
                Some(first_index) => {
                    is_possible_duplicate = true;
                    possible_duplicates[*first_index] = true;
                    possible_duplicates[i] = true;
                },
            }
        }
        if !is_possible_duplicate {
            return transactions;
        }

        let hash_and_authenticators: Vec<_> = possible_duplicates
            .into_par_iter()
            .zip(&transactions)
            .with_min_len(25)
            .map(|(need_hash, txn)| match need_hash {
                true => Some((txn.clone().committed_hash(), txn.authenticator())),
                false => None,
            })
            .collect();
        let mut seen_hashes = HashSet::new();
        let mut num_duplicates: usize = 0;
        let duplicates: Vec<_> = hash_and_authenticators
            .into_iter()
            .map(|maybe_hash| match maybe_hash {
                None => false,
                Some(hash_and_authenticator) => {
                    if seen_hashes.insert(hash_and_authenticator) {
                        false
                    } else {
                        num_duplicates += 1;
                        true
                    }
                },
            })
            .collect();
        TXN_DEDUP_FILTERED.observe(num_duplicates as f64);
        if num_duplicates == 0 {
            return transactions;
        }

        transactions
            .into_iter()
            .zip(duplicates)
            .filter_map(|(txn, is_duplicate)| if is_duplicate { None } else { Some(txn) })
            .collect()
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
