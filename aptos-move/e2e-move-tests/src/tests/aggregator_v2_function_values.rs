// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::executor::FakeExecutor;
use aptos_transaction_simulation::Account;
use aptos_types::{move_utils::MemberId, transaction::ExecutionStatus};
use claims::assert_ok;
use move_core_types::{
    account_address::AccountAddress, parser::parse_struct_tag, vm_status::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Deserialize, Serialize)]
struct Aggregator {
    value: u64,
    max_value: u64,
}

#[derive(Deserialize, Serialize)]
struct Counter {
    aggregator: Aggregator,
}

fn assert_counter_value_eq(h: &MoveHarness, account: &Account, value: u64) {
    let tag = parse_struct_tag("0x1::proxy::Counter").unwrap();
    let counter = h.read_resource::<Counter>(account.address(), tag).unwrap();
    assert_eq!(counter.aggregator.value, value);
}

fn initialize(h: &mut MoveHarness) {
    let build_options = BuildOptions::move_2().set_latest_language();
    let path = common::test_dir_path("aggregator_v2.data/function_values");

    let framework_account = h.aptos_framework_account();
    let status = h.publish_package_with_options(&framework_account, path.as_path(), build_options);
    assert_success!(status);
}

#[test]
fn test_function_value_is_applied_to_aggregator() {
    let mut h = MoveHarness::new_with_executor(FakeExecutor::from_head_genesis().set_parallel());
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    initialize(&mut h);

    let mut value = 100;
    let status = h.run_entry_function(
        &acc,
        MemberId::from_str("0x1::proxy::initialize").unwrap(),
        vec![],
        vec![bcs::to_bytes(&value).unwrap()],
    );
    assert_success!(status);

    assert_counter_value_eq(&h, &acc, value);

    let add_1 = MemberId::from_str("0x1::function_values::add_1").unwrap();
    let add_2 = MemberId::from_str("0x1::function_values::add_2").unwrap();
    let add_3 = MemberId::from_str("0x1::function_values::add_3").unwrap();

    let mut txns = vec![];
    for i in 0..33 {
        value += i;
        let args = vec![bcs::to_bytes(&i).unwrap()];
        let txn = match i % 3 {
            0 => h.create_entry_function(&acc, add_1.clone(), vec![], args),
            1 => h.create_entry_function(&acc, add_2.clone(), vec![], args),
            2 => h.create_entry_function(&acc, add_3.clone(), vec![], args),
            _ => unreachable!(),
        };
        txns.push(txn);
    }

    let statuses = h.run_block(txns);
    for status in statuses {
        assert_success!(status);
    }
    assert_counter_value_eq(&h, &acc, value);
}

#[test]
fn test_function_value_captures_aggregator_is_not_storable() {
    let mut h = MoveHarness::new_with_executor(FakeExecutor::from_head_genesis().set_parallel());
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    initialize(&mut h);

    let status = h.run_entry_function(
        &acc,
        MemberId::from_str("0x1::function_store::try_initialize_should_abort").unwrap(),
        vec![],
        vec![bcs::to_bytes(&100_u64).unwrap()],
    );
    assert_vm_status!(status, StatusCode::VALUE_SERIALIZATION_ERROR);

    let status = h.run_entry_function(
        &acc,
        MemberId::from_str("0x1::function_store::function_store_does_not_exist").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(status);

    let output = h.execute_view_function(
        MemberId::from_str("0x1::function_store::view_function_store_exists").unwrap(),
        vec![],
        vec![bcs::to_bytes(acc.address()).unwrap()],
    );
    let exists =
        bcs::from_bytes::<bool>(&output.values.expect("View function should succeed")[0]).unwrap();
    assert!(!exists);
}

#[test]
fn test_function_value_captures_aggregator() {
    let mut h = MoveHarness::new_with_executor(FakeExecutor::from_head_genesis().set_parallel());
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    initialize(&mut h);

    let mut value = 100;
    let status = h.run_entry_function(
        &acc,
        MemberId::from_str("0x1::proxy::initialize").unwrap(),
        vec![],
        vec![bcs::to_bytes(&value).unwrap()],
    );
    assert_success!(status);
    assert_counter_value_eq(&h, &acc, value);

    let increment = 100;
    value += increment;
    let status = h.run_entry_function(
        &acc,
        MemberId::from_str("0x1::capturing::capture_aggregator").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&increment).unwrap(),
            bcs::to_bytes(&value).unwrap(),
        ],
    );
    assert_success!(status);

    for name in [
        "to_bytes_with_captured_aggregator",
        "to_string_with_captured_aggregator",
        "emit_event_with_captured_aggregator",
    ] {
        let status = h.run_entry_function(
            &acc,
            MemberId::from_str(&format!("0x1::capturing::{name}")).unwrap(),
            vec![],
            vec![],
        );
        assert_vm_status!(status, StatusCode::VALUE_SERIALIZATION_ERROR);
    }

    let status = h.run_entry_function(
        &acc,
        MemberId::from_str("0x1::capturing::serialized_size_with_captured_aggregator").unwrap(),
        vec![],
        vec![],
    );

    // Note: the native function remaps the error and aborts with this code.
    let status = assert_ok!(status.as_kept_status());
    assert!(matches!(status, ExecutionStatus::MoveAbort {
        code: 453,
        ..
    }));
}
