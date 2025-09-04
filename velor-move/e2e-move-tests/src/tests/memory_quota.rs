// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use velor_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::value::MoveValue;

// TODO(Gas): This test has been disabled since the particularly attack it uses can no longer
//            be carried out due to the increase in execution costs.
//            Revisit and decide whether we should remove this test or rewrite it in another way.
/*
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
*/

#[test]
fn deeply_nested_structs() {
    let mut h = MoveHarness::new();

    h.modify_gas_schedule(|gas_params| {
        gas_params.vm.txn.memory_quota = 10_000_000.into();
        gas_params.vm.txn.max_execution_gas = 100_000_000_000.into();
    });

    // Publish the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("memory_quota.data/nested_struct"),
    ));

    // Initialize
    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::very_nested_structure::init").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);

    // Create nested structs as table entries
    for _i in 0..5 {
        let result = h.run_entry_function(
            &acc,
            str::parse("0xbeef::very_nested_structure::add").unwrap(),
            vec![],
            vec![MoveValue::U64(2000).simple_serialize().unwrap()],
        );
        assert_success!(result);
    }

    // Try to load the whole table -- this should succeed
    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::very_nested_structure::read_all").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);

    // Forward 2 hours to activate TimedFeatureFlag::FixMemoryUsageTracking
    // Now attempting to load the whole table shall result in an execution failure (memory limit hit)
    h.new_epoch();
    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::very_nested_structure::read_all").unwrap(),
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

#[test]
fn add_vec_to_table() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("memory_quota.data/table_and_vec"),
    ));

    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::just_under_quota").unwrap(),
        vec![],
        vec![],
    );
    // Should fail when trying to destroy a non-empty table.
    assert!(matches!(
        result,
        TransactionStatus::Keep(ExecutionStatus::MoveAbort { .. })
    ));

    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::just_above_quota").unwrap(),
        vec![],
        vec![],
    );
    // Should run out of memory before trying to destroy a non-empty table.
    assert!(matches!(
        result,
        TransactionStatus::Keep(ExecutionStatus::ExecutionFailure { .. })
    ));
}
