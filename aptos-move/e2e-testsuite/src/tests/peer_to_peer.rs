// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::{
    account::Account, common_transactions::peer_to_peer_txn, executor::FakeExecutor,
};
use aptos_types::{
    account_config::{DepositEvent, WithdrawEvent},
    transaction::{ExecutionStatus, SignedTransaction, TransactionOutput, TransactionStatus},
};
use rstest::rstest;
use std::{convert::TryFrom, time::Instant};

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    stateless_sender,
    case(false, false, false),
    case(true, false, false),
    case(true, true, false),
    case(true, true, true)
)]
fn single_peer_to_peer_with_event(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
    stateless_sender: bool,
) {
    ::aptos_logger::Logger::init_for_testing();
    let mut executor = FakeExecutor::from_head_genesis();

    // create and publish a sender with 1_000_000 coins and a receiver with 100_000 coins
    let sender = if stateless_sender {
        executor.create_raw_account_data(1_000_000, None)
    } else {
        executor.create_raw_account_data(1_000_000, Some(10))
    };
    let receiver = executor.create_raw_account_data(100_000, Some(10));
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    let transfer_amount = 1_000;
    let txn = peer_to_peer_txn(
        sender.account(),
        receiver.account(),
        if use_orderless_transactions {
            None
        } else {
            Some(10)
        },
        transfer_amount,
        0,
        executor.get_block_time_seconds(), // current_time
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

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
    let updated_sender_balance = executor
        .read_apt_fungible_store_resource(sender.account())
        .expect("sender balance must exist");
    let updated_receiver_balance = executor
        .read_apt_fungible_store_resource(receiver.account())
        .expect("receiver balance must exist");
    assert_eq!(receiver_balance, updated_receiver_balance.balance());
    assert_eq!(sender_balance, updated_sender_balance.balance());

    // Check sequence number based on transaction type and sender state
    if stateless_sender && use_orderless_transactions {
        // For stateless senders with orderless transactions, no account resource should be created
        assert!(
            executor.read_account_resource(sender.account()).is_none(),
            "sender resource shouldn't be created with an orderless transaction on stateless account"
        );
    } else {
        let updated_sender = executor
            .read_account_resource(sender.account())
            .expect("sender must exist");
        if use_orderless_transactions {
            assert_eq!(10, updated_sender.sequence_number());
        } else {
            assert_eq!(11, updated_sender.sequence_number());
        }
    }
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    stateless_sender,
    case(false, false, false),
    case(true, false, false),
    case(true, true, false),
    case(true, true, true)
)]
fn few_peer_to_peer_with_event(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
    stateless_sender: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    // create and publish a sender with 3_000_000 coins and a receiver with 3_000_000 coins
    let sender = if stateless_sender {
        executor.create_raw_account_data(3_000_000, None)
    } else {
        executor.create_raw_account_data(3_000_000, Some(10))
    };
    let receiver = executor.create_raw_account_data(3_000_000, Some(10));
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    let transfer_amount = 1_000;

    // execute transaction
    // For orderless transactions, only execute one transaction since multiple
    // transactions from the same sender would have the same nonce
    let txns: Vec<SignedTransaction> = if use_orderless_transactions {
        vec![peer_to_peer_txn(
            sender.account(),
            receiver.account(),
            None,
            transfer_amount,
            0,
            executor.get_block_time_seconds(), // current_time
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )]
    } else {
        vec![
            peer_to_peer_txn(
                sender.account(),
                receiver.account(),
                Some(10),
                transfer_amount,
                0,
                executor.get_block_time_seconds(), // current_time
                use_txn_payload_v2_format,
                use_orderless_transactions,
            ),
            peer_to_peer_txn(
                sender.account(),
                receiver.account(),
                Some(11),
                transfer_amount,
                0,
                executor.get_block_time_seconds(), // current_time
                use_txn_payload_v2_format,
                use_orderless_transactions,
            ),
            peer_to_peer_txn(
                sender.account(),
                receiver.account(),
                Some(12),
                transfer_amount,
                0,
                executor.get_block_time_seconds(), // current_time
                use_txn_payload_v2_format,
                use_orderless_transactions,
            ),
            peer_to_peer_txn(
                sender.account(),
                receiver.account(),
                Some(13),
                transfer_amount,
                0,
                executor.get_block_time_seconds(), // current_time
                use_txn_payload_v2_format,
                use_orderless_transactions,
            ),
        ]
    };
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
            .expect("receiver balance must exist");
        executor.apply_write_set(txn_output.write_set());

        // check that numbers in stored DB are correct
        let sender_balance = original_sender_balance.balance() - transfer_amount;
        let receiver_balance = original_receiver_balance.balance() + transfer_amount;
        let updated_sender_balance = executor
            .read_apt_fungible_store_resource(sender.account())
            .expect("sender balance must exist");
        let updated_receiver_balance = executor
            .read_apt_fungible_store_resource(receiver.account())
            .expect("receiver balance must exist");
        assert_eq!(receiver_balance, updated_receiver_balance.balance());
        assert_eq!(sender_balance, updated_sender_balance.balance());

        // Check sequence number based on transaction type and sender state
        if stateless_sender && use_orderless_transactions {
            // For stateless senders with orderless transactions, no account resource should be created
            assert!(
                executor.read_account_resource(sender.account()).is_none(),
                "sender resource shouldn't be created with an orderless transaction on stateless account"
            );
        } else {
            let updated_sender = executor
                .read_account_resource(sender.account())
                .expect("sender must exist");
            if use_orderless_transactions {
                assert_eq!(10, updated_sender.sequence_number());
            } else {
                assert_eq!(11 + idx as u64, updated_sender.sequence_number());
            }
        }
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
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> (Vec<TxnInfo>, Vec<SignedTransaction>) {
    let mut txns: Vec<SignedTransaction> = Vec::new();
    let mut txns_info: Vec<TxnInfo> = Vec::new();
    // loop through all transactions and let each transfer the same amount to the next one
    let count = accounts.len();
    for i in 0..count {
        let sender = &accounts[i];
        let seq_num = if use_orderless_transactions {
            None
        } else {
            let sender_resource = executor
                .read_account_resource(sender)
                .expect("sender must exist");
            Some(sender_resource.sequence_number())
        };
        let receiver = &accounts[(i + 1) % count];

        let txn = peer_to_peer_txn(
            sender,
            receiver,
            seq_num,
            transfer_amount,
            0,
            executor.get_block_time_seconds(), // current_time
            use_txn_payload_v2_format,
            use_orderless_transactions,
        );
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
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> (Vec<TxnInfo>, Vec<SignedTransaction>) {
    let mut txns: Vec<SignedTransaction> = Vec::new();
    let mut txns_info: Vec<TxnInfo> = Vec::new();
    // grab account 0 as a sender
    let sender = &accounts[0];
    let base_seq_num = if use_orderless_transactions {
        None
    } else {
        let sender_resource = executor
            .read_account_resource(sender)
            .expect("sender must exist");
        Some(sender_resource.sequence_number())
    };
    // loop through all transactions and let each transfer the same amount to the next one
    let count = accounts.len();
    // For orderless transactions, only create one transaction since multiple
    // transactions from the same sender would have the same nonce
    let receivers_to_process = if use_orderless_transactions {
        1
    } else {
        count - 1
    };
    for (i, receiver) in accounts
        .iter()
        .enumerate()
        .take(receivers_to_process + 1)
        .skip(1)
    {
        let seq_num = if use_orderless_transactions {
            None
        } else {
            Some(base_seq_num.unwrap() + i as u64 - 1)
        };

        let txn = peer_to_peer_txn(
            sender,
            receiver,
            seq_num,
            transfer_amount,
            0,
            executor.get_block_time_seconds(), // current_time
            use_txn_payload_v2_format,
            use_orderless_transactions,
        );
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
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> (Vec<TxnInfo>, Vec<SignedTransaction>) {
    let mut txns: Vec<SignedTransaction> = Vec::new();
    let mut txns_info: Vec<TxnInfo> = Vec::new();
    // grab account 0 as a receiver
    let receiver = &accounts[0];
    // loop through all transactions and let each transfer the same amount to the receiver
    let count = accounts.len();
    for sender in accounts.iter().take(count).skip(1) {
        let seq_num = if use_orderless_transactions {
            None
        } else {
            let sender_resource = executor
                .read_account_resource(sender)
                .expect("sender must exist");
            Some(sender_resource.sequence_number())
        };

        let txn = peer_to_peer_txn(
            sender,
            receiver,
            seq_num,
            transfer_amount,
            0,
            executor.get_block_time_seconds(), // current_time
            use_txn_payload_v2_format,
            use_orderless_transactions,
        );
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
    use_orderless_transactions: bool,
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

        // For orderless transactions, sequence number should not change
        if use_orderless_transactions {
            assert_eq!(sender_seq_num, updated_sender.sequence_number());
        } else {
            assert_eq!(sender_seq_num + 1, updated_sender.sequence_number());
        }
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

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn cycle_peer_to_peer(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut executor = FakeExecutor::from_head_genesis();

    let account_size = 100usize;
    let initial_balance = 2_000_000u64;
    let initial_seq_num = 10u64;
    let accounts = executor.create_accounts(account_size, initial_balance, Some(initial_seq_num));

    // set up the transactions
    let transfer_amount = 1_000;
    let (txns_info, txns) = create_cyclic_transfers(
        &executor,
        &accounts,
        transfer_amount,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

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

    check_and_apply_transfer_output(
        &mut executor,
        &txns_info,
        &output,
        use_orderless_transactions,
    );
    print_accounts(&executor, &accounts);
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn cycle_peer_to_peer_multi_block(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();

    let account_size = 100usize;
    let initial_balance = 1_000_000u64;
    let initial_seq_num = 10u64;
    let accounts = executor.create_accounts(account_size, initial_balance, Some(initial_seq_num));

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
            use_txn_payload_v2_format,
            use_orderless_transactions,
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
        check_and_apply_transfer_output(
            &mut executor,
            &txns_info,
            &output,
            use_orderless_transactions,
        );
        range_left = (range_left + cycle) % account_size;
    }
    println!("EXECUTION TIME: {}", execution_time);
    print_accounts(&executor, &accounts);
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn one_to_many_peer_to_peer(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut executor = FakeExecutor::from_head_genesis();

    let account_size = 100usize;
    let initial_balance = 100_000_000u64;
    let initial_seq_num = 10u64;
    let accounts = executor.create_accounts(account_size, initial_balance, Some(initial_seq_num));

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
            use_txn_payload_v2_format,
            use_orderless_transactions,
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
        let expected_output_len = if use_orderless_transactions {
            1
        } else {
            cycle - 1
        };
        assert_eq!(expected_output_len, output.len());
        check_and_apply_transfer_output(
            &mut executor,
            &txns_info,
            &output,
            use_orderless_transactions,
        );
        range_left = (range_left + cycle) % account_size;
    }
    println!("EXECUTION TIME: {}", execution_time);
    print_accounts(&executor, &accounts);
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn many_to_one_peer_to_peer(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut executor = FakeExecutor::from_head_genesis();

    let account_size = 100usize;
    let initial_balance = 1_000_000u64;
    let initial_seq_num = 10u64;
    let accounts = executor.create_accounts(account_size, initial_balance, Some(initial_seq_num));

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
            use_txn_payload_v2_format,
            use_orderless_transactions,
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
        // many_to_one doesn't have the same issue as one_to_many since each sender only sends one transaction
        assert_eq!(cycle - 1, output.len());
        check_and_apply_transfer_output(
            &mut executor,
            &txns_info,
            &output,
            use_orderless_transactions,
        );
        range_left = (range_left + cycle) % account_size;
    }
    println!("EXECUTION TIME: {}", execution_time);
    print_accounts(&executor, &accounts);
}
