// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for enum variant counts

use crate::{assert_success, tests::common::test_dir_path, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};

#[test]
fn test_enum_storage() {
    let mut h = MoveHarness::new();
    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package(&acc, &test_dir_path("enum_variants_count.data"),));

    // Create the transaction but don't run it directly
    let txn = h.create_entry_function(
        &acc,
        str::parse("0xbeef::VersionModule::store_version").unwrap(),
        vec![],
        vec![],
    );

    // Run the transaction and get detailed output
    let output = h.run_block_get_output(vec![txn]).pop().unwrap();
    assert_eq!(
        *output.status(),
        TransactionStatus::Keep(ExecutionStatus::Success)
    );

    // Create the transaction but don't run it directly
    let txn = h.create_entry_function(
        &acc,
        str::parse("0xbeef::VersionModule::get_version").unwrap(),
        vec![],
        vec![],
    );

    // Run the transaction and get detailed output
    let output = h.run_block_get_output(vec![txn]).pop().unwrap();
    assert_eq!(
        *output.status(),
        TransactionStatus::Keep(ExecutionStatus::Success)
    );
}
