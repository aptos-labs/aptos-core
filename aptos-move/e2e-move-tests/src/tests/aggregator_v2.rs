// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregator_v2::{
        initialize, verify_copy_snapshot, verify_copy_string_snapshot, verify_string_concat,
        verify_string_snapshot_concat,
    },
    assert_success,
    tests::common,
    MoveHarness,
};
use aptos_language_e2e_tests::account::Account;

fn setup() -> (MoveHarness, Account) {
    initialize(common::test_dir_path("aggregator.data/pack"))
}

#[test]
fn test_copy_snapshot() {
    let (mut h, acc) = setup();
    let txn = verify_copy_snapshot(&mut h, &acc);
    assert_success!(h.run(txn));
}

#[test]
fn test_copy_string_snapshot() {
    let (mut h, acc) = setup();
    let txn = verify_copy_string_snapshot(&mut h, &acc);
    assert_success!(h.run(txn));
}

#[test]
fn test_string_concat() {
    let (mut h, acc) = setup();
    let txn = verify_string_concat(&mut h, &acc);
    assert_success!(h.run(txn));
}

#[test]
fn test_string_snapshot_concat() {
    let (mut h, acc) = setup();
    let txn = verify_string_snapshot_concat(&mut h, &acc);
    assert_success!(h.run(txn));
}
