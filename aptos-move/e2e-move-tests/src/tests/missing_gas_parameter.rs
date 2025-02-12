// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{tests::common, MoveHarness};
use aptos_types::{account_address::AccountAddress, transaction::TransactionStatus};
use move_core_types::vm_status::StatusCode;
use rstest::rstest;

#[rstest(stateless_account,
    case(true),
    case(false),
)]
fn missing_gas_parameter(stateless_account: bool) {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap(), if stateless_account { None } else { Some(0)});
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
