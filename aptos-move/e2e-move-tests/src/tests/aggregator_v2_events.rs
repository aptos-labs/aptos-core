// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    assert_success,
    tests::{aggregator_v2::EAGGREGATOR_OVERFLOW, common},
    MoveHarness,
};
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, SignedTransaction, TransactionOutput},
};
use claims::{assert_matches, assert_ok, assert_some};
use move_core_types::{
    account_address::AccountAddress, ident_str, language_storage::ModuleId,
    parser::parse_struct_tag, vm_status::AbortLocation,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Aggregator {
    value: u64,
    max_value: u64,
}

#[derive(Deserialize, Serialize)]
struct AggregatorSnapshot {
    value: u64,
}

#[derive(Deserialize, Serialize)]
struct Counter {
    value: Aggregator,
}

#[derive(Deserialize, Serialize)]
struct Event {
    value: AggregatorSnapshot,
}

fn publish_test_package(h: &mut MoveHarness, aptos_framework_account: &Account) {
    let path_buf = common::test_dir_path("aggregator_v2.data/pack");
    assert_success!(h.publish_package_cache_building(aptos_framework_account, path_buf.as_path()));
}

fn create_test_txn(
    h: &mut MoveHarness,
    aptos_framework_account: &Account,
    name: &str,
) -> SignedTransaction {
    h.create_entry_function(
        aptos_framework_account,
        str::parse(name).unwrap(),
        vec![],
        vec![bcs::to_bytes(aptos_framework_account.address()).unwrap()],
    )
}

fn run_entry_functions<F: Fn(ExecutionStatus)>(
    h: &mut MoveHarness,
    aptos_framework_account: &Account,
    func_names: Vec<&str>,
    check_status: F,
) -> Vec<TransactionOutput> {
    publish_test_package(h, aptos_framework_account);

    // Make sure aggregators are enabled, so that we can test.
    h.enable_features(
        vec![
            FeatureFlag::AGGREGATOR_V2_API,
            FeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS,
            FeatureFlag::RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET,
        ],
        vec![],
    );

    let txns = func_names
        .into_iter()
        .map(|name| create_test_txn(h, aptos_framework_account, name))
        .collect();

    let outputs = h.run_block_get_output(txns);
    for output in outputs.iter() {
        let execution_status = assert_ok!(output.status().as_kept_status());
        check_status(execution_status);
    }
    outputs
}

fn check_events(outputs: Vec<TransactionOutput>) {
    // We always emit fee statement event. Currently this is hardcoded
    // for existing test case.
    assert_eq!(outputs[0].events().len(), 1);
    assert_eq!(outputs[1].events().len(), 1);

    assert_eq!(outputs[2].events().len(), 2);
    let event = assert_ok!(bcs::from_bytes::<Event>(
        outputs[2].events()[0].event_data()
    ));
    assert_eq!(event.value.value, 2);

    assert_eq!(outputs[3].events().len(), 1);
    assert_eq!(outputs[4].events().len(), 1);

    assert_eq!(outputs[5].events().len(), 2);
    let event = assert_ok!(bcs::from_bytes::<Event>(
        outputs[5].events()[0].event_data()
    ));
    assert_eq!(event.value.value, 5);

    assert_eq!(outputs[6].events().len(), 1);

    assert_eq!(outputs[7].events().len(), 2);
    let event = assert_ok!(bcs::from_bytes::<Event>(
        outputs[7].events()[0].event_data()
    ));
    assert_eq!(event.value.value, 6);

    assert_eq!(outputs[8].events().len(), 1);
    assert_eq!(outputs[9].events().len(), 1);
}

#[test]
fn test_events_v1_with_snapshots() {
    let func_names = vec![
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::test_emit_event_v1",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::test_increment_counter_and_emit_event_v1",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::test_emit_event_v1",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::increment_counter",
    ];

    let mut h = MoveHarness::new();
    let aptos_framework_account = h.aptos_framework_account();
    let check_status = |status: ExecutionStatus| {
        assert_matches!(status, ExecutionStatus::Success);
    };
    let outputs = run_entry_functions(&mut h, &aptos_framework_account, func_names, check_status);

    // Counter has been incremented 8 times.
    let c = assert_some!(h.read_resource::<Counter>(
        aptos_framework_account.address(),
        parse_struct_tag("0x1::events_with_aggregators::Counter").unwrap(),
    ));
    assert_eq!(c.value.value, 8);
    check_events(outputs);
}

#[test]
fn test_events_v2_with_snapshots() {
    let func_names = vec![
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::test_emit_event_v2",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::test_increment_counter_and_emit_event_v2",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::test_emit_event_v2",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::increment_counter",
    ];

    let mut h = MoveHarness::new();
    let aptos_framework_account = h.aptos_framework_account();
    let check_status = |status: ExecutionStatus| {
        assert_matches!(status, ExecutionStatus::Success);
    };
    let outputs = run_entry_functions(&mut h, &aptos_framework_account, func_names, check_status);

    let c = assert_some!(h.read_resource::<Counter>(
        aptos_framework_account.address(),
        parse_struct_tag("0x1::events_with_aggregators::Counter").unwrap(),
    ));
    assert_eq!(c.value.value, 8);
    check_events(outputs);
}

fn check_failed_output(failed_output: TransactionOutput) {
    let status = assert_ok!(failed_output.status().as_kept_status());
    let aggregator_location =
        ModuleId::new(AccountAddress::ONE, ident_str!("aggregator_v2").to_owned());
    if let ExecutionStatus::MoveAbort {
        location: AbortLocation::Module(id),
        code: EAGGREGATOR_OVERFLOW,
        info: Some(_),
    } = status
    {
        assert_eq!(id, aggregator_location);
    } else {
        unreachable!("Expected Move abort, got {:?}", status);
    }
    assert_eq!(failed_output.events().len(), 1);
}

#[test]
fn test_events_with_snapshots_not_emitted_on_abort() {
    let func_names = vec![
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::increment_counter",
        "0x1::events_with_aggregators::increment_counter",
        // Overflows here.
        "0x1::events_with_aggregators::test_increment_counter_and_emit_event_v1",
        "0x1::events_with_aggregators::test_increment_counter_and_emit_event_v2",
    ];

    let mut h = MoveHarness::new();
    let aptos_framework_account = h.aptos_framework_account();
    let check_status = |_: ExecutionStatus| {
        // no-op.
    };
    let mut outputs =
        run_entry_functions(&mut h, &aptos_framework_account, func_names, check_status);

    let c = assert_some!(h.read_resource::<Counter>(
        aptos_framework_account.address(),
        parse_struct_tag("0x1::events_with_aggregators::Counter").unwrap(),
    ));
    assert_eq!(c.value.value, 10);

    // Last two transactions overflow.
    check_failed_output(outputs.pop().unwrap());
    check_failed_output(outputs.pop().unwrap());
}
