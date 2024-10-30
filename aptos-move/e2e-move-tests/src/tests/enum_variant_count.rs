// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for enum variant counts

// Note[Orderless]: Done
use crate::{assert_success, tests::common::test_dir_path, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
use rstest::rstest;

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_enum_storage(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
    stateless_account: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("publisher".to_string(), *acc.address());
    assert_success!(h.publish_package_with_options(
        &acc,
        &test_dir_path("enum_variants_count.data"),
        build_options
    ));

    // Create the transaction but don't run it directly
    let txn = h.create_entry_function(
        &acc,
        str::parse(&format!("{}::VersionModule::store_version", acc.address())).unwrap(),
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
        str::parse(&format!("{}::VersionModule::get_version", acc.address())).unwrap(),
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
