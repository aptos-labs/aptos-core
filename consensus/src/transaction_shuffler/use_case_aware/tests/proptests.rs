// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::use_case_aware::{
    iterator::ShuffledTransactionIterator,
    tests::{into_txns, Account, Contract, Transaction},
    Config,
};
use itertools::Itertools;
use proptest::{collection::vec, prelude::*};
use std::collections::HashMap;

fn txn_indices_by_account(txns: &[Transaction]) -> HashMap<u8, Vec<usize>> {
    txns.iter()
        .map(|txn| (txn.sender.0, txn.original_idx))
        .into_group_map()
}

proptest! {
    #[test]
    fn test_no_panic(
        txns in vec(any::<(Contract, Account)>(), 0..100)
        .prop_map(into_txns),
        sender_factor in 0..100usize,
        platform_factor in 0..100usize,
        user_contract_factor in 0..100usize,
    ) {
        let num_txns = txns.len();
        let txns_by_account = txn_indices_by_account(&txns);

        let config = Config {
            sender_spread_factor: sender_factor,
            platform_use_case_spread_factor: platform_factor,
            user_use_case_spread_factor: user_contract_factor,
        };

        let shuffled_txns = ShuffledTransactionIterator::new(config)
            .extended_with(txns)
            .collect_vec();

        prop_assert_eq!(
            txn_indices_by_account(&shuffled_txns),
            txns_by_account
        );

        let txn_indices = shuffled_txns.into_iter().map(|txn| txn.original_idx).sorted().collect_vec();
        prop_assert_eq!(txn_indices, (0..num_txns).collect_vec());
    }
}
