// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::vm_status::StatusCode::DEPENDENCY_LIMIT_REACHED;

#[test]
fn exceeding_max_num_dependencies_test_with_stateful_sender() {
    let mut h = MoveHarness::new();
    let stateful_acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap(), Some(0));
    exceeding_max_num_dependencies(&mut h, stateful_acc);
}

#[test]
fn exceeding_max_num_dependencies_test_with_stateless_sender() {
    let mut h = MoveHarness::new();
    let stateless_acc = h.new_account_at(AccountAddress::from_hex_literal("0xdafe").unwrap(), None);  
    exceeding_max_num_dependencies(&mut h, stateless_acc);
}

fn exceeding_max_num_dependencies(h: &mut MoveHarness, acc: Account) {
    h.modify_gas_schedule(|gas_params| {
        gas_params.vm.txn.max_num_dependencies = 2.into();
    });

    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("dependencies.data/p1"),)
    );
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("dependencies.data/p2"),)
    );

    // Publishing should fail
    let res =
        h.publish_package_cache_building(&acc, &common::test_dir_path("dependencies.data/p3"));
    assert!(matches!(
        res,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            DEPENDENCY_LIMIT_REACHED
        )))
    ));

    // Publishing should succeed if we increase the limit
    h.modify_gas_schedule(|gas_params| {
        gas_params.vm.txn.max_num_dependencies = 3.into();
    });
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("dependencies.data/p3"),)
    );

    // Should be able to use module
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::m3::run").unwrap(),
        vec![],
        vec![],
    ));

    // Should no longer be able to use module if we decrease the limit again
    h.modify_gas_schedule(|gas_params| {
        gas_params.vm.txn.max_num_dependencies = 2.into();
    });
    let res = h.run_entry_function(&acc, str::parse("0xcafe::m3::run").unwrap(), vec![], vec![]);
    assert!(matches!(
        res,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            DEPENDENCY_LIMIT_REACHED
        )))
    ));
}

#[test]
fn exceeding_max_dependency_size_test_with_stateful_sender() {
    let mut h = MoveHarness::new();
    let stateful_acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap(), Some(0));
    exceeding_max_dependency_size(&mut h, stateful_acc);
}

#[test]
fn exceeding_max_dependency_size_test_with_stateless_sender() {
    let mut h = MoveHarness::new();
    let stateless_acc = h.new_account_at(AccountAddress::from_hex_literal("0xdafe").unwrap(), None);  
    exceeding_max_dependency_size(&mut h, stateless_acc);
}

fn exceeding_max_dependency_size(h: &mut MoveHarness, acc: Account) {
    h.modify_gas_schedule(|gas_params| {
        gas_params.vm.txn.max_total_dependency_size = 260.into();
    });

    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("dependencies.data/p1"),)
    );
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("dependencies.data/p2"),)
    );

    // Publishing should fail
    let res =
        h.publish_package_cache_building(&acc, &common::test_dir_path("dependencies.data/p3"));
    assert!(matches!(
        res,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            DEPENDENCY_LIMIT_REACHED
        )))
    ));

    // Publishing should succeed if we increase the limit
    h.modify_gas_schedule(|gas_params| {
        gas_params.vm.txn.max_total_dependency_size = 1000000.into();
    });
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("dependencies.data/p3"),)
    );

    // Should be able to use module
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::m3::run").unwrap(),
        vec![],
        vec![],
    ));

    // Should no longer be able to use module if we decrease the limit again
    h.modify_gas_schedule(|gas_params| {
        gas_params.vm.txn.max_total_dependency_size = 220.into();
    });
    let res = h.run_entry_function(&acc, str::parse("0xcafe::m3::run").unwrap(), vec![], vec![]);
    assert!(matches!(
        res,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            DEPENDENCY_LIMIT_REACHED
        )))
    ));
}
