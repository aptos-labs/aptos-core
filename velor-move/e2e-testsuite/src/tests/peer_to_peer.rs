// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_language_e2e_tests::{
    account::Account, common_transactions::peer_to_peer_txn, executor::FakeExecutor,
};
use velor_types::{
    account_config::{DepositEvent, WithdrawEvent},
    transaction::{ExecutionStatus, SignedTransaction, TransactionOutput, TransactionStatus},
};
use std::{convert::TryFrom, time::Instant};

#[test]
fn single_peer_to_peer_with_event() {
    ::velor_logger::Logger::init_for_testing();
    let mut executor = FakeExecutor::from_head_genesis();
    // create and publish a sender with 1_000_000 coins and a receiver with 100_000 coins
    let sender = executor.create_raw_account_data(1_000_000, 10);
    let receiver = executor.create_raw_account_data(100_000, 10);
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    let transfer_amount = 1_000;
    let txn = peer_to_peer_txn(sender.account(), receiver.account(), 10, transfer_amount, 0);

    // execute transaction
    let output = executor.execute_transaction(txn);
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success)
    );

    executor.apply_write_set(output.write_set());

    // check that numbers in stored DB are correct
    let sender_balance = 1_000_000 - transfer_amount;
    let receiver_balance = 100_000 + transfer_amount;
    let updated_sender = executor
        .read_account_resource(sender.account())
        .expect("sender must exist");
    let updated_sender_balance = executor
        .read_apt_fungible_store_resource(sender.account())
        .expect("sender balance must exist");
    let updated_receiver_balance = executor
        .read_apt_fungible_store_resource(receiver.account())
        .expect("receiver balance must exist");
    assert_eq!(receiver_balance, updated_receiver_balance.balance());
    assert_eq!(sender_balance, updated_sender_balance.balance());
    assert_eq!(11, updated_sender.sequence_number());
}

#[test]
fn few_peer_to_peer_with_event() {
    let mut executor = FakeExecutor::from_head_genesis();

    // create and publish a sender with 3_000_000 coins and a receiver with 3_000_000 coins
    let sender = executor.create_raw_account_data(3_000_000, 10);
    let receiver = executor.create_raw_account_data(3_000_000, 10);
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    let transfer_amount = 1_000;

    // execute transaction
    let txns: Vec<SignedTransaction> = vec![
        peer_to_peer_txn(sender.account(), receiver.account(), 10, transfer_amount, 0),
        peer_to_peer_txn(sender.account(), receiver.account(), 11, transfer_amount, 0),
        peer_to_peer_txn(sender.account(), receiver.account(), 12, transfer_amount, 0),
        peer_to_peer_txn(sender.account(), receiver.account(), 13, transfer_amount, 0),
    ];
    let output = executor.execute_block(txns).unwrap();
    for (idx, txn_output) in output.iter().enumerate() {
        assert_eq!(
            txn_output.status(),
            &TransactionStatus::Keep(ExecutionStatus::Success)
        );

        // check events
        for event in txn_output.events() {
            if let Ok(payload) = WithdrawEvent::try_from(event) {
                assert_eq!(transfer_amount, payload.amount());
            } else if let Ok(payload) = DepositEvent::try_from(event) {
                if payload.amount() == 0 {
                    continue;
                }
                assert_eq!(transfer_amount, payload.amount());
            } else {
                assert!(event.v2().is_ok());
            }
        }

        let original_sender_balance = executor
            .read_apt_fungible_store_resource(sender.account())
            .expect("sender balance must exist");
        let original_receiver_balance = executor
            .read_apt_fungible_store_resource(receiver.account())
            .expect("receiver balcne must exist");
        executor.apply_write_set(txn_output.write_set());

        // check that numbers in stored DB are correct
        let sender_balance = original_sender_balance.balance() - transfer_amount;
        let receiver_balance = original_receiver_balance.balance() + transfer_amount;
        let updated_sender = executor
            .read_account_resource(sender.account())
            .expect("sender must exist");
        let updated_sender_balance = executor
            .read_apt_fungible_store_resource(sender.account())
            .expect("sender balance must exist");
        let updated_receiver_balance = executor
            .read_apt_fungible_store_resource(receiver.account())
            .expect("receiver balance must exist");
        assert_eq!(receiver_balance, updated_receiver_balance.balance());
        assert_eq!(sender_balance, updated_sender_balance.balance());
        assert_eq!(11 + idx as u64, updated_sender.sequence_number());
    }
}

// Holder for transaction data; arguments to transactions.
pub(crate) struct TxnInfo {
    pub sender: Account,
    pub receiver: Account,
    pub transfer_amount: u64,
}

impl TxnInfo {
    fn new(sender: &Account, receiver: &Account, transfer_amount: u64) -> Self {
        TxnInfo {
            sender: sender.clone(),
            receiver: receiver.clone(),
            transfer_amount,
        }
    }
}

// Create a cyclic transfer around a slice of Accounts.
// Each Account makes a transfer for the same amount to the next Account.
pub(crate) fn create_cyclic_transfers(
    executor: &FakeExecutor,
    accounts: &[Account],
    transfer_amount: u64,
) -> (Vec<TxnInfo>, Vec<SignedTransaction>) {
    let mut txns: Vec<SignedTransaction> = Vec::new();
    let mut txns_info: Vec<TxnInfo> = Vec::new();
    // loop through all transactions and let each transfer the same amount to the next one
    let count = accounts.len();
    for i in 0..count {
        let sender = &accounts[i];
        let sender_resource = executor
            .read_account_resource(sender)
            .expect("sender must exist");
        let seq_num = sender_resource.sequence_number();
        let receiver = &accounts[(i + 1) % count];

        let txn = peer_to_peer_txn(sender, receiver, seq_num, transfer_amount, 0);
        txns.push(txn);
        txns_info.push(TxnInfo::new(sender, receiver, transfer_amount));
    }
    (txns_info, txns)
}

// Create a one to many transfer around a slice of Accounts.
// The first account is the payer and all others are receivers.
fn create_one_to_many_transfers(
    executor: &FakeExecutor,
    accounts: &[Account],
    transfer_amount: u64,
) -> (Vec<TxnInfo>, Vec<SignedTransaction>) {
    let mut txns: Vec<SignedTransaction> = Vec::new();
    let mut txns_info: Vec<TxnInfo> = Vec::new();
    // grab account 0 as a sender
    let sender = &accounts[0];
    let sender_resource = executor
        .read_account_resource(sender)
        .expect("sender must exist");
    let seq_num = sender_resource.sequence_number();
    // loop through all transactions and let each transfer the same amount to the next one
    let count = accounts.len();
    for (i, receiver) in accounts.iter().enumerate().take(count).skip(1) {
        // let receiver = &accounts[i];

        let txn = peer_to_peer_txn(sender, receiver, seq_num + i as u64 - 1, transfer_amount, 0);
        txns.push(txn);
        txns_info.push(TxnInfo::new(sender, receiver, transfer_amount));
    }
    (txns_info, txns)
}

// Create a many to one transfer around a slice of Accounts.
// The first account is the receiver and all others are payers.
fn create_many_to_one_transfers(
    executor: &FakeExecutor,
    accounts: &[Account],
    transfer_amount: u64,
) -> (Vec<TxnInfo>, Vec<SignedTransaction>) {
    let mut txns: Vec<SignedTransaction> = Vec::new();
    let mut txns_info: Vec<TxnInfo> = Vec::new();
    // grab account 0 as a sender
    let receiver = &accounts[0];
    // loop through all transactions and let each transfer the same amount to the next one
    let count = accounts.len();
    for sender in accounts.iter().take(count).skip(1) {
        //let sender = &accounts[i];
        let sender_resource = executor
            .read_account_resource(sender)
            .expect("sender must exist");
        let seq_num = sender_resource.sequence_number();

        let txn = peer_to_peer_txn(sender, receiver, seq_num, transfer_amount, 0);
        txns.push(txn);
        txns_info.push(TxnInfo::new(sender, receiver, transfer_amount));
    }
    (txns_info, txns)
}

// Verify a transfer output.
// Checks that sender and receiver in a peer to peer transaction are in proper
// state after a successful transfer.
// The transaction arguments are provided in txn_args.
// Apply the WriteSet to the data store.
pub(crate) fn check_and_apply_transfer_output(
    executor: &mut FakeExecutor,
    txn_args: &[TxnInfo],
    output: &[TransactionOutput],
) {
    let count = output.len();
    for i in 0..count {
        let txn_info = &txn_args[i];
        let sender = &txn_info.sender;
        let receiver = &txn_info.receiver;
        let transfer_amount = txn_info.transfer_amount;
        let sender_resource = executor
            .read_account_resource(sender)
            .expect("sender must exist");
        let sender_balance = executor
            .read_apt_fungible_store_resource(sender)
            .expect("sender balance must exist");
        let sender_initial_balance = sender_balance.balance();
        let sender_seq_num = sender_resource.sequence_number();
        let receiver_initial_balance = executor
            .read_apt_fungible_store_resource(receiver)
            .expect("receiver balance must exist")
            .balance();

        // apply single transaction to DB
        let txn_output = &output[i];
        executor.apply_write_set(txn_output.write_set());

        // check that numbers stored in DB are correct
        let sender_balance = sender_initial_balance - transfer_amount;
        let receiver_balance = receiver_initial_balance + transfer_amount;
        let updated_sender = executor
            .read_account_resource(sender)
            .expect("sender must exist");
        let updated_sender_balance = executor
            .read_apt_fungible_store_resource(sender)
            .expect("sender balance must exist");
        let updated_receiver_balance = executor
            .read_apt_fungible_store_resource(receiver)
            .expect("receiver balance must exist");
        assert_eq!(receiver_balance, updated_receiver_balance.balance());
        assert_eq!(sender_balance, updated_sender_balance.balance());
        assert_eq!(sender_seq_num + 1, updated_sender.sequence_number());
    }
}

// simple utility to print all account to visually inspect account data
fn print_accounts(executor: &FakeExecutor, accounts: &[Account]) {
    for account in accounts {
        let account_resource = executor
            .read_account_resource(account)
            .expect("sender must exist");
        println!("{:?}", account_resource);
    }
}

#[test]
fn cycle_peer_to_peer() {
    let mut executor = FakeExecutor::from_head_genesis();
    let account_size = 100usize;
    let initial_balance = 2_000_000u64;
    let initial_seq_num = 10u64;
    let accounts = executor.create_accounts(account_size, initial_balance, initial_seq_num);

    // set up the transactions
    let transfer_amount = 1_000;
    let (txns_info, txns) = create_cyclic_transfers(&executor, &accounts, transfer_amount);

    // execute transaction
    let mut execution_time = 0u128;
    let now = Instant::now();
    let output = executor.execute_block(txns).unwrap();
    execution_time += now.elapsed().as_nanos();
    println!("EXECUTION TIME: {}", execution_time);
    for txn_output in &output {
        assert_eq!(
            txn_output.status(),
            &TransactionStatus::Keep(ExecutionStatus::Success)
        );
    }
    assert_eq!(accounts.len(), output.len());

    check_and_apply_transfer_output(&mut executor, &txns_info, &output);
    print_accounts(&executor, &accounts);
}

#[test]
fn cycle_peer_to_peer_multi_block() {
    let mut executor = FakeExecutor::from_head_genesis();
    let account_size = 100usize;
    let initial_balance = 1_000_000u64;
    let initial_seq_num = 10u64;
    let accounts = executor.create_accounts(account_size, initial_balance, initial_seq_num);

    // set up the transactions
    let transfer_amount = 1_000;
    let block_count = 5u64;
    let cycle = account_size / (block_count as usize);
    let mut range_left = 0usize;
    let mut execution_time = 0u128;
    for _i in 0..block_count {
        range_left = if range_left + cycle >= account_size {
            account_size - cycle
        } else {
            range_left
        };
        let (txns_info, txns) = create_cyclic_transfers(
            &executor,
            &accounts[range_left..range_left + cycle],
            transfer_amount,
        );

        // execute transaction
        let now = Instant::now();
        let output = executor.execute_block(txns).unwrap();
        execution_time += now.elapsed().as_nanos();
        for txn_output in &output {
            assert_eq!(
                txn_output.status(),
                &TransactionStatus::Keep(ExecutionStatus::Success)
            );
        }
        assert_eq!(cycle, output.len());
        check_and_apply_transfer_output(&mut executor, &txns_info, &output);
        range_left = (range_left + cycle) % account_size;
    }
    println!("EXECUTION TIME: {}", execution_time);
    print_accounts(&executor, &accounts);
}

#[test]
fn one_to_many_peer_to_peer() {
    let mut executor = FakeExecutor::from_head_genesis();
    let account_size = 100usize;
    let initial_balance = 100_000_000u64;
    let initial_seq_num = 10u64;
    let accounts = executor.create_accounts(account_size, initial_balance, initial_seq_num);

    // set up the transactions
    let transfer_amount = 1_000;
    let block_count = 2u64;
    let cycle = account_size / (block_count as usize);
    let mut range_left = 0usize;
    let mut execution_time = 0u128;
    for _i in 0..block_count {
        range_left = if range_left + cycle >= account_size {
            account_size - cycle
        } else {
            range_left
        };
        let (txns_info, txns) = create_one_to_many_transfers(
            &executor,
            &accounts[range_left..range_left + cycle],
            transfer_amount,
        );

        // execute transaction
        let now = Instant::now();
        let output = executor.execute_block(txns).unwrap();
        execution_time += now.elapsed().as_nanos();
        for txn_output in &output {
            assert_eq!(
                txn_output.status(),
                &TransactionStatus::Keep(ExecutionStatus::Success)
            );
        }
        assert_eq!(cycle - 1, output.len());
        check_and_apply_transfer_output(&mut executor, &txns_info, &output);
        range_left = (range_left + cycle) % account_size;
    }
    println!("EXECUTION TIME: {}", execution_time);
    print_accounts(&executor, &accounts);
}

#[test]
fn many_to_one_peer_to_peer() {
    let mut executor = FakeExecutor::from_head_genesis();
    let account_size = 100usize;
    let initial_balance = 1_000_000u64;
    let initial_seq_num = 10u64;
    let accounts = executor.create_accounts(account_size, initial_balance, initial_seq_num);

    // set up the transactions
    let transfer_amount = 1_000;
    let block_count = 2u64;
    let cycle = account_size / (block_count as usize);
    let mut range_left = 0usize;
    let mut execution_time = 0u128;
    for _i in 0..block_count {
        range_left = if range_left + cycle >= account_size {
            account_size - cycle
        } else {
            range_left
        };
        let (txns_info, txns) = create_many_to_one_transfers(
            &executor,
            &accounts[range_left..range_left + cycle],
            transfer_amount,
        );

        // execute transaction
        let now = Instant::now();
        let output = executor.execute_block(txns).unwrap();
        execution_time += now.elapsed().as_nanos();
        for txn_output in &output {
            assert_eq!(
                txn_output.status(),
                &TransactionStatus::Keep(ExecutionStatus::Success)
            );
        }
        assert_eq!(cycle - 1, output.len());
        check_and_apply_transfer_output(&mut executor, &txns_info, &output);
        range_left = (range_left + cycle) % account_size;
    }
    println!("EXECUTION TIME: {}", execution_time);
    print_accounts(&executor, &accounts);
}
