// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{tests::common, MoveHarness};
use velor_types::{account_address::AccountAddress, transaction::TransactionStatus};
use move_core_types::vm_status::StatusCode;

#[test]
fn missing_gas_parameter() {
    let mut h = MoveHarness::new();

    h.modify_gas_schedule_raw(|gas_schedule| {
        let idx = gas_schedule
            .entries
            .iter()
            .position(|(key, _val)| key == "instr.add")
            .unwrap();
        gas_schedule.entries.remove(idx);
    });

    // Load the code
    let acc = h.new_account_with_balance_at(AccountAddress::from_hex_literal("0xbeef").unwrap(), 0);
    let txn_status = h.publish_package(&acc, &common::test_dir_path("common.data/do_nothing"));
    assert!(matches!(
        txn_status,
        TransactionStatus::Discard(StatusCode::VM_STARTUP_FAILURE)
    ))
}
