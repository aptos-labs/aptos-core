// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
use language_e2e_tests::{
    account::Account, common_transactions::create_account_txn, current_function_name,
    executor::FakeExecutor,
};

#[test]
fn create_account() {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());

    // create and publish a sender with 1_000_000 coins
    let sender = Account::new_aptos_root();
    let new_account = executor.create_raw_account();

    // define the arguments to the create account transaction
    let initial_amount = 0;
    let txn = create_account_txn(&sender, &new_account, 0);

    // execute transaction
    let output = executor.execute_transaction(txn);
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success)
    );
    executor.apply_write_set(output.write_set());

    // check that numbers in stored DB are correct
    let updated_sender = executor
        .read_account_resource(&sender)
        .expect("sender must exist");

    let updated_receiver_balance = executor
        .read_coin_store_resource(&new_account)
        .expect("receiver balance must exist");
    assert_eq!(initial_amount, updated_receiver_balance.coin());
    assert_eq!(1, updated_sender.sequence_number());
}
