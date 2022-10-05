// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};

#[test]
fn push_u128s_onto_vector() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("memory_quota.data/vec_push_u128"),
    ));

    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::just_under_quota").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);

    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::just_above_quota").unwrap(),
        vec![],
        vec![],
    );
    assert!(matches!(
        result,
        TransactionStatus::Keep(ExecutionStatus::ExecutionFailure { .. })
    ));
}

#[test]
fn clone_large_vectors() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("memory_quota.data/clone_vec"),));

    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::just_under_quota").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);

    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::just_above_quota").unwrap(),
        vec![],
        vec![],
    );
    assert!(matches!(
        result,
        TransactionStatus::Keep(ExecutionStatus::ExecutionFailure { .. })
    ));
}
