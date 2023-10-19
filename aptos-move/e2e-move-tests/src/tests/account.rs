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

    let sender = Account::new();
    let receiver = h.new_account_with_balance_and_sequence_number(100_000, 0);

    let txn = sender
        .transaction()
        .payload(aptos_account_transfer(*receiver.address(), 10))
        .sequence_number(0)
        .sign();

    // lite_account will be created automatically.
    let status = h.run(txn);

    assert_err_eq!(
        status.status(),
        StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE
    );
}
