// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::MoveHarness;
use aptos_cached_packages::aptos_stdlib::aptos_account_transfer;
use aptos_types::transaction::ExecutionStatus;
use claims::assert_ok_eq;
use fail::FailScenario;
use move_core_types::vm_status::StatusCode;

#[test]
fn failed_transaction_cleanup_charges_gas() {
    // Fail in user transaction so we can run failed transaction cleanup.
    let scenario = FailScenario::setup();
    assert!(fail::has_failpoints());
    fail::cfg("aptos_vm::execute_script_or_entry_function", "return()").unwrap();
    assert!(!fail::list().is_empty());

    // Actual transaction is correct, so that we get to the failpoint.
    let mut h = MoveHarness::new();
    let sender = h.new_account_with_balance_and_sequence_number(1_000_000, 10);
    let receiver = h.new_account_with_balance_and_sequence_number(1_000_000, 10);
    let txn = sender
        .transaction()
        .sequence_number(10)
        .payload(aptos_account_transfer(*receiver.address(), 1))
        .sign();
    let output = h.run_block_get_output(vec![txn]).pop().unwrap();

    // After failures in user transactions, even if these are invariant violations,
    // gas should still be charged.
    assert_ne!(output.gas_used(), 0);
    assert!(!output.status().is_discarded());
    assert_ok_eq!(
        output.status().status(),
        ExecutionStatus::MiscellaneousError(Some(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR))
    );

    scenario.teardown();
}
