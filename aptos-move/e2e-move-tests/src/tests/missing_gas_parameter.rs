// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{tests::common, MoveHarness};
use aptos_types::{account_address::AccountAddress, transaction::TransactionStatus};
use move_core_types::vm_status::StatusCode;

#[test]
fn missing_gas_parameter_with_stateful_sender() {
    let mut h = MoveHarness::new();
    let stateless_acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap(), Some(0));
    missing_gas_parameter(&mut h, acc);
}

#[test]
fn missing_gas_parameter_with_stateless_sender() {
    let mut h = MoveHarness::new();
    let stateless_acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap(), Some(0));
    missing_gas_parameter(&mut h, acc);
}

fn missing_gas_parameter(h: &mut MoveHarness, acc: Account) {

    h.modify_gas_schedule_raw(|gas_schedule| {
        let idx = gas_schedule
            .entries
            .iter()
            .position(|(key, _val)| key == "instr.add")
            .unwrap();
        gas_schedule.entries.remove(idx);
    });

    let txn_status = h.publish_package(&acc, &common::test_dir_path("common.data/do_nothing"));
    assert!(matches!(
        txn_status,
        TransactionStatus::Discard(StatusCode::VM_STARTUP_FAILURE)
    ))
}
