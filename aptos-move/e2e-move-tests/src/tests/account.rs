// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::MoveHarness;
use aptos_cached_packages::aptos_stdlib::aptos_account_transfer;
use aptos_language_e2e_tests::account::Account;
use claims::assert_err_eq;
use move_core_types::vm_status::StatusCode;

#[test]
fn non_existent_sender() {
    let mut h = MoveHarness::new();
    // Sender account is not stored on-chain.
    let sender = Account::new();
    let receiver = h.new_account_with_balance_and_sequence_number(100_000, 0);

    let txn = sender
        .transaction()
        .payload(aptos_account_transfer(*receiver.address(), 10))
        .sign();

    let output = h.run(txn);
    assert_err_eq!(output.status(), StatusCode::SENDING_ACCOUNT_DOES_NOT_EXIST,);
}
