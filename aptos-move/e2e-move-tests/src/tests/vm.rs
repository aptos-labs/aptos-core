// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::MoveHarness;
use aptos_cached_packages::aptos_stdlib::aptos_account_transfer;
use aptos_types::{
    state_store::state_key::StateKey, transaction::ExecutionStatus, write_set::WriteOp,
};
use aptos_vm::AptosVM;
use aptos_vm_environment::environment::AptosEnvironment;
use claims::{assert_ok_eq, assert_some};
use move_core_types::vm_status::{StatusCode, VMStatus};
use rstest::rstest;

// Note[Orderless]: We are ignoring the (stateless_account, sequence number based transaction) case when running this test.
// This is because, if the sender is stateless when the epilogue is run, the epilogue will create the 0x1::Account resource for the sender.
// But, the epilogue::finish method adds a check that no creations are in writeset for epilogue. So, this test fails for that case.
// In reality, it is not an issue because when a stateless sender sends a sequence number transaction, the prologue either discards
// the transaction or creates the 0x1::Account resource for the sender.

// Make sure verification and invariant violation errors are kept.
#[rstest(status_code, stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    // case(StatusCode::TYPE_MISMATCH, true, false, false),
    // case(StatusCode::TYPE_MISMATCH, true, true, false),
    case(StatusCode::TYPE_MISMATCH, true, true, true),
    case(StatusCode::TYPE_MISMATCH, false, false, false),
    case(StatusCode::TYPE_MISMATCH, false, true, false),
    case(StatusCode::TYPE_MISMATCH, false, true, true),
    // case(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, true, false, false),
    // case(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, true, true, false),
    case(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, true, true, true),
    case(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, false, false, false),
    case(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, false, true, false),
    case(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, false, true, true),
)]
fn failed_transaction_cleanup_charges_gas(
    status_code: StatusCode,
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let sender = h.new_account_with_balance_and_sequence_number(
        1_000_000,
        if stateless_account { None } else { Some(0) },
    );
    let receiver = h.new_account_with_balance_and_sequence_number(1_000_000, Some(10));

    let max_gas_amount = 100_000;
    let txn = sender
        .transaction()
        .sequence_number(0)
        .max_gas_amount(max_gas_amount)
        .payload(aptos_account_transfer(*receiver.address(), 1))
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();

    let state_view = h.executor.get_state_view();
    let env = AptosEnvironment::new(&state_view);
    let vm = AptosVM::new(&env, state_view);

    let balance = 10_000;
    let output = vm
        .test_failed_transaction_cleanup(
            VMStatus::error(status_code, None),
            &txn,
            state_view,
            balance,
        )
        .1;
    let write_set: Vec<(&StateKey, &WriteOp)> = output
        .concrete_write_set_iter()
        .map(|(k, v)| (k, assert_some!(v)))
        .collect();
    if use_orderless_transactions {
        // Sequence number is not updated for orderless transactions. So, writeset is empty
        assert!(write_set.is_empty());
    } else {
        assert!(!write_set.is_empty());
    }
    assert_eq!(output.gas_used(), max_gas_amount - balance);
    assert!(!output.status().is_discarded());
    assert_ok_eq!(
        output.status().as_kept_status(),
        ExecutionStatus::MiscellaneousError(Some(status_code))
    );
}
