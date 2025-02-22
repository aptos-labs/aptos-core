// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::MoveHarness;
use aptos_cached_packages::aptos_stdlib::aptos_account_transfer;
use aptos_language_e2e_tests::account::Account;
use aptos_types::transaction::ExecutionStatus;
use claims::{assert_err_eq, assert_matches};
use move_core_types::vm_status::StatusCode;
use rstest::rstest;

#[test]
fn non_existent_sender_running_seq_number_txns() {
    let mut h = MoveHarness::new();

    let sender = Account::new();
    let receiver = h.new_account_with_balance_and_sequence_number(100_000, Some(0));

    let txn = sender
        .transaction()
        .payload(aptos_account_transfer(*receiver.address(), 10))
        .sequence_number(0)
        .sign();

    let status = h.run(txn);
    // TODO[Orderless]: This error code might change as we now create 0x1::Account resource if it doesn't exist.
    assert_err_eq!(status.status(), StatusCode::SENDING_ACCOUNT_DOES_NOT_EXIST);
}

#[test]
fn non_existent_sender_running_nonce_txns() {
    let mut h = MoveHarness::new();

    if h.use_orderless_transactions {
        let sender = Account::new();
        let receiver = h.new_account_with_balance_and_sequence_number(100_000, Some(0));

        let txn = sender
            .transaction()
            .payload(aptos_account_transfer(*receiver.address(), 10))
            .upgrade_payload(h.use_txn_payload_v2_format, h.use_orderless_transactions)
            .sign();

        let status = h.run(txn);
        assert_err_eq!(status.status(), StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE);
    }
}

#[rstest(receiver_stateless_account,
    case(true),
    case(false),
)]
fn stateless_sender_with_account_balance(receiver_stateless_account: bool) {
    let mut h = MoveHarness::new_with_flags(true, true);
    let sender = h.new_account_with_balance_and_sequence_number(100_000, None);
    let receiver = h.new_account_with_balance_and_sequence_number(100_000, if receiver_stateless_account { None } else { Some(0) });

    let txn = sender
        .transaction()
        .payload(aptos_account_transfer(*receiver.address(), 10))
        .upgrade_payload(true, true)
        .sign();

    let status = h.run(txn);
    assert_matches!(status.status(), Ok(ExecutionStatus::Success));
}