// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_types::transaction::TransactionStatus;
use move_core_types::vm_status::StatusCode;
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
fn missing_gas_parameter(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    h.modify_gas_schedule_raw(|gas_schedule| {
        let idx = gas_schedule
            .entries
            .iter()
            .position(|(key, _val)| key == "instr.add")
            .unwrap();
        gas_schedule.entries.remove(idx);
    });

    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("publisher".to_string(), *acc.address());
    let txn_status = h.publish_package_with_options(
        &acc,
        &common::test_dir_path("common.data/do_nothing"),
        build_options,
    );
    assert!(matches!(
        txn_status,
        TransactionStatus::Discard(StatusCode::VM_STARTUP_FAILURE)
    ))
}
