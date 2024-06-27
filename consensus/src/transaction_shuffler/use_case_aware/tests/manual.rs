// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::use_case_aware::{
    iterator::ShuffledTransactionIterator,
    tests,
    tests::{Account, Contract},
    Config,
};
use itertools::Itertools;

const PP: Contract = Contract::Platform;
const OO: Contract = Contract::Others;
const C1: Contract = Contract::User(0xF1);
const C2: Contract = Contract::User(0xF2);
const C3: Contract = Contract::User(0xF3);
const A1: Account = Account(1);
const A2: Account = Account(2);
const A3: Account = Account(3);
const A4: Account = Account(4);

fn assert_shuffle_result(
    config: Config,
    txns: impl IntoIterator<Item = (Contract, Account)>,
    expected_order: impl IntoIterator<Item = usize>,
) {
    let txns = tests::into_txns(txns);
    let actual_order = ShuffledTransactionIterator::new(config)
        .extended_with(txns)
        .map(|txn| txn.original_idx)
        .collect_vec();
    let expected_order = expected_order.into_iter().collect_vec();
    assert_eq!(actual_order, expected_order, "actual != expected");
}

fn three_senders_txns() -> [(Contract, Account); 10] {
    [
        // 5 txns from A1
        (PP, A1),
        (OO, A1),
        (C1, A1),
        (C2, A1),
        (C3, A1),
        // 3 txns from A2
        (PP, A2),
        (PP, A2),
        (PP, A2),
        // 2 txns from A3
        (C1, A3),
        (C1, A3),
    ]
}

#[test]
fn test_no_spreading() {
    let config = Config {
        sender_spread_factor: 0,
        platform_use_case_spread_factor: 0,
        user_use_case_spread_factor: 0,
    };
    let txns = three_senders_txns();

    assert_shuffle_result(config, txns, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn test_spread_by_sender_1() {
    let config = Config {
        sender_spread_factor: 1,
        // ignore use case conflicts
        platform_use_case_spread_factor: 0,
        // ignore use case conflicts
        user_use_case_spread_factor: 0,
    };
    let txns = three_senders_txns();

    assert_shuffle_result(config, txns, [0, 5, 1, 6, 2, 7, 3, 8, 4, 9]);
}

#[test]
fn test_spread_by_sender_by_large_factor() {
    for sender_spread_factor in [2, 3, 4] {
        let config = Config {
            sender_spread_factor,
            platform_use_case_spread_factor: 0,
            user_use_case_spread_factor: 0,
        };
        let txns = three_senders_txns();

        assert_shuffle_result(config, txns, [0, 5, 8, 1, 6, 9, 2, 7, 3, 4]);
    }
}

fn three_contracts_txns() -> [(Contract, Account); 10] {
    [
        // 5 txns from C1
        (C1, A1),
        (C1, A1),
        (C1, A1),
        (C1, A1),
        (C1, A1),
        // 3 txns from C2
        (C2, A2),
        (C2, A2),
        (C2, A2),
        // 2 txns from C3
        (C3, A3),
        (C3, A3),
    ]
}

#[test]
fn test_spread_by_use_case_1() {
    let config = Config {
        sender_spread_factor: 0,
        platform_use_case_spread_factor: 0,
        user_use_case_spread_factor: 1,
    };
    let txns = three_contracts_txns();

    assert_shuffle_result(config, txns, [0, 5, 1, 6, 2, 7, 3, 8, 4, 9]);
}

#[test]
fn test_spread_by_use_case_by_large_factor() {
    for user_use_case_spread_factor in [2, 3, 4] {
        let config = Config {
            sender_spread_factor: 0,
            platform_use_case_spread_factor: 0,
            user_use_case_spread_factor,
        };
        let txns = three_contracts_txns();

        assert_shuffle_result(config, txns, [0, 5, 8, 1, 6, 9, 2, 7, 3, 4]);
    }
}

fn user_and_platform_use_cases() -> [(Contract, Account); 10] {
    [
        // 5 txns from C1
        (C1, A1),
        (C1, A1),
        (C1, A1),
        (C1, A1),
        (C1, A1),
        // 3 txns from C2
        (PP, A2),
        (PP, A2),
        (PP, A2),
        // 2 txns from C3
        (PP, A3),
        (PP, A3),
    ]
}

#[test]
fn test_platform_txn_priority_0() {
    let config = Config {
        sender_spread_factor: 0,
        platform_use_case_spread_factor: 0,
        user_use_case_spread_factor: 3,
    };
    let txns = user_and_platform_use_cases();

    assert_shuffle_result(config, txns, [0, 5, 6, 7, 1, 8, 9, 2, 3, 4]);
}

#[test]
fn test_platform_txn_priority_1() {
    let config = Config {
        sender_spread_factor: 0,
        platform_use_case_spread_factor: 1,
        user_use_case_spread_factor: 3,
    };
    let txns = user_and_platform_use_cases();

    assert_shuffle_result(config, txns, [0, 5, 6, 1, 7, 8, 2, 9, 3, 4]);
}

#[test]
fn test_spread_sender_within_use_case() {
    let config = Config {
        sender_spread_factor: 2,
        platform_use_case_spread_factor: 0,
        user_use_case_spread_factor: 1,
    };
    let txns = [
        // 5 txns from C1
        (C1, A1),
        (C1, A1),
        (C1, A2),
        (C1, A2),
        (C1, A2),
        // 3 txns from C2
        (C2, A3),
        (C2, A3),
        (C2, A3),
        (C2, A4),
        (C2, A4),
    ];

    assert_shuffle_result(config, txns, [0, 5, 2, 8, 1, 6, 3, 9, 4, 7]);
}
