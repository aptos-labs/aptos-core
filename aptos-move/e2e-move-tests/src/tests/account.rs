// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::MoveHarness;
use aptos_cached_packages::aptos_stdlib::aptos_account_transfer;
use aptos_language_e2e_tests::account::Account;
use aptos_types::transaction::ExecutionStatus;
use claims::{assert_err_eq, assert_matches};
use move_core_types::vm_status::StatusCode;
use rstest::rstest;

#[rstest(use_txn_payload_v2_format, use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true),
)]
fn non_existent_sender_with_high_seq_number(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);

    let sender = Account::new();
    let receiver = h.new_account_with_balance_and_sequence_number(100_000, Some(0));

    let txn = sender
        .transaction()
        .payload(aptos_account_transfer(*receiver.address(), 10))
        .sequence_number(1)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();

    let status = h.run(txn);
    if use_orderless_transactions {
        assert!(!status.is_discarded());
    } else {
        assert_err_eq!(status.status(), StatusCode::SEQUENCE_NUMBER_TOO_NEW);
    }
}

#[rstest(use_txn_payload_v2_format, use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true),
)]
fn non_existent_sender(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);

    let sender = Account::new();
    let receiver = h.new_account_with_balance_and_sequence_number(100_000, Some(0));

    let txn = sender
        .transaction()
        .payload(aptos_account_transfer(*receiver.address(), 10))
        .sequence_number(0)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign();

    let status = h.run(txn);
    assert!(!status.is_discarded());
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