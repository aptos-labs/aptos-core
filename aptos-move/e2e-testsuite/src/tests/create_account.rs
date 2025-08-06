// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::{
    account::Account, common_transactions::peer_to_peer_txn, current_function_name,
    executor::FakeExecutor,
};
use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
use rstest::rstest;

#[rstest(use_txn_payload_v2_format, case(false), case(true))]
fn create_account_with_seq_num_based_txn(use_txn_payload_v2_format: bool) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());

    let sender = Account::new_aptos_root();
    let new_account = executor.create_raw_account();

    let initial_amount = 1;
    let txn = peer_to_peer_txn(
        &sender,
        &new_account,
        Some(0),
        1,
        1,
        u64::MAX - 60,
        use_txn_payload_v2_format,
        false,
    );

    // execute transaction
    let output = executor.execute_transaction(txn);
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success)
    );
    executor.apply_write_set(output.write_set());
    // check that numbers in stored DB are correct
    let updated_receiver_balance = executor
        .read_apt_fungible_store_resource(&new_account)
        .expect("receiver balance must exist");
    assert_eq!(initial_amount, updated_receiver_balance.balance());
    let updated_sender = executor
        .read_account_resource(&sender)
        .expect("sender must exist");
    assert_eq!(1, updated_sender.sequence_number());
}

#[rstest(stateless_sender, case(true), case(false))]
fn create_account_with_orderless_txn(stateless_sender: bool) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());

    // create and publish a sender with 1_000_000 coins
    let sender = executor.create_raw_account();
    let sender = executor.store_and_fund_account(
        sender,
        1_000_000,
        if stateless_sender { None } else { Some(0) },
    );
    let new_account = executor.create_raw_account();

    let initial_amount = 1;
    let txn = peer_to_peer_txn(
        sender.account(),
        &new_account,
        None,
        1,
        1,
        executor.get_block_time_seconds(),
        true,
        true,
    );

    // execute transaction
    let output = executor.execute_transaction(txn);
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success)
    );
    executor.apply_write_set(output.write_set());
    // check that numbers in stored DB are correct
    let updated_receiver_balance = executor
        .read_apt_fungible_store_resource(&new_account)
        .expect("receiver balance must exist");
    assert_eq!(initial_amount, updated_receiver_balance.balance());
    if stateless_sender {
        assert!(
            executor.read_account_resource(sender.account()).is_none(),
            "sender resource shouldn't be created with an orderless transaction"
        );
    } else {
        let updated_sender = executor
            .read_account_resource(sender.account())
            .expect("sender must exist");
        assert_eq!(0, updated_sender.sequence_number());
    }
}
