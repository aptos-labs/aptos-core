// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_transaction_builder::experimental_stdlib::encode_create_account_script_function;
use diem_types::vm_status::StatusCode;
use language_e2e_tests::{account::Account, current_function_name, executor::FakeExecutor};

// Make sure we can start the experimental genesis
#[test]
fn experimental_genesis_runs() {
    FakeExecutor::from_experimental_genesis();
}

// Make sure that we can execute transactions with the experimental genesis
#[test]
fn experimental_genesis_execute_txn_successful() {
    let mut executor = FakeExecutor::from_experimental_genesis();
    executor.set_golden_file(current_function_name!());
    let new_account = executor.create_raw_account();
    let new_new_account = executor.create_raw_account();
    let dr_account = Account::new_diem_root();
    let txn = dr_account
        .transaction()
        .payload(encode_create_account_script_function(
            *new_account.address(),
            new_account.auth_key_prefix(),
        ))
        .sequence_number(0)
        .sign();
    executor.execute_and_apply(txn);

    // Other accounts can create accounts, no role checks
    let txn = new_account
        .transaction()
        .payload(encode_create_account_script_function(
            *new_new_account.address(),
            new_new_account.auth_key_prefix(),
        ))
        .sequence_number(0)
        .sign();
    executor.execute_and_apply(txn);
}

// Make sure that we can handle prologue errors from the non-DPN account module
#[test]
fn experimental_genesis_execute_txn_non_existent_sender() {
    let mut executor = FakeExecutor::from_experimental_genesis();
    executor.set_golden_file(current_function_name!());
    let new_account = executor.create_raw_account();
    let txn = new_account
        .transaction()
        .payload(encode_create_account_script_function(
            *new_account.address(),
            new_account.auth_key_prefix(),
        ))
        .sequence_number(0)
        .sign();
    let output = &executor.execute_transaction(txn);
    assert_eq!(
        output.status().status(),
        Err(StatusCode::SENDING_ACCOUNT_DOES_NOT_EXIST),
    );
}
