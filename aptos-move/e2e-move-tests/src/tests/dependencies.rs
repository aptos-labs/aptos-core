// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::vm_status::StatusCode::DEPENDENCY_LIMIT_REACHED;
use rstest::rstest;

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn exceeding_max_num_dependencies(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap(), if stateless_account { None } else { Some(0) });

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

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn exceeding_max_dependency_size(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap(), if stateless_account { None } else { Some(0) });
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
