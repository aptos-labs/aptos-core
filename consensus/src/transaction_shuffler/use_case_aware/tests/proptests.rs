// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{into_txns, Account, Contract};
use crate::transaction_shuffler::use_case_aware::{iterator::ShuffledTransactionIterator, Config};
use itertools::Itertools;
use proptest::{collection::vec, prelude::*};

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

        let config = Config {
            sender_spread_factor: sender_factor,
            platform_use_case_spread_factor: platform_factor,
            user_use_case_spread_factor: user_contract_factor,
        };

        let mut txn_indices = ShuffledTransactionIterator::new(config)
            .extended_with(txns)
            .map(|txn| txn.original_idx)
            .collect_vec();

        txn_indices.sort_unstable();
        prop_assert_eq!(txn_indices, (0..num_txns).collect_vec());
    }
}
