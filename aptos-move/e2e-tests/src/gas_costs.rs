// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Gas costs for common transactions.

use crate::{
    account::{Account, AccountData},
    common_transactions::{create_account_txn, peer_to_peer_txn},
    executor::FakeExecutor,
};
use aptos_types::transaction::SignedTransaction;
use once_cell::sync::Lazy;
// TODO[Orderless]: This file currently computes gas costs for seq number based transactions with txn payload v1 format.
// Add corresponding gas costs for txn payload v2 format, and nonce based transactions.

/// The gas each transaction is configured to reserve. If the gas available in the account,
/// converted to microaptos, falls below this threshold, transactions are expected to fail with
/// an insufficient balance.
pub const TXN_RESERVED: u64 = 500_000;

/// The gas cost of a first time create-account transaction.
///
/// This includes the cost of the event counter creation which makes the transaction more
/// expensive. All such transactions are expected to cost the same gas.
pub static CREATE_ACCOUNT_FIRST: Lazy<u64> = Lazy::new(|| {
    let mut executor = FakeExecutor::from_head_genesis();
    let sender = AccountData::new(1_000_000, Some(10));
    executor.add_account_data(&sender);
    let receiver = Account::new();

    let txn = create_account_txn(
        sender.account(),
        &receiver,
        Some(10),
        executor.get_block_time_seconds(),
        false,
        false,
    );
    compute_gas_used(txn, &mut executor)
});

/// The gas cost of a create-account transaction.
///
/// This is the cost after the event counter has been created.
/// All such transactions are expected to cost the same gas.
pub static CREATE_ACCOUNT_NEXT: Lazy<u64> = Lazy::new(|| {
    let mut executor = FakeExecutor::from_head_genesis();
    let sender = AccountData::new(1_000_000, Some(10));
    executor.add_account_data(&sender);

    let txns = vec![
        create_account_txn(
            sender.account(),
            &Account::new(),
            Some(10),
            executor.get_block_time_seconds(),
            false,
            false,
        ),
        create_account_txn(
            sender.account(),
            &Account::new(),
            Some(11),
            executor.get_block_time_seconds(),
            false,
            false,
        ),
    ];
    let output = &executor
        .execute_block(txns)
        .expect("The VM should not fail to startup");
    output[1].gas_used()
});

/// The gas cost of a create-account transaction where the sender has an insufficient balance.
///
/// This includes the cost of the event counter creation. As such the cost of the transaction
/// would be higher and the balance required must be higher.
/// All such transactions are expected to cost the same gas.
pub static CREATE_ACCOUNT_TOO_LOW_FIRST: Lazy<u64> = Lazy::new(|| {
    let mut executor = FakeExecutor::from_head_genesis();
    // The gas amount is the minimum that needs to be reserved, so use a value that's
    // clearly higher than that.
    let balance = TXN_RESERVED + 10_000;
    let sender = AccountData::new(balance, Some(10));
    executor.add_account_data(&sender);
    let receiver = Account::new();

    let txn = create_account_txn(
        sender.account(),
        &receiver,
        Some(10),
        executor.get_block_time_seconds(),
        false,
        false,
    );
    compute_gas_used(txn, &mut executor)
});

/// The gas cost of a create-account transaction where the sender has an insufficient balance.
///
/// This is the cost after the event counter has been created.
/// All such transactions are expected to cost the same gas.
pub static CREATE_ACCOUNT_TOO_LOW_NEXT: Lazy<u64> = Lazy::new(|| {
    let mut executor = FakeExecutor::from_head_genesis();
    // The gas amount is the minimum that needs to be reserved, so use a value that's
    // clearly higher than that.
    let balance = (2 * TXN_RESERVED) + 10_000;
    let sender = AccountData::new(balance, Some(10));
    executor.add_account_data(&sender);

    let txns = vec![
        create_account_txn(
            sender.account(),
            &Account::new(),
            Some(10),
            executor.get_block_time_seconds(),
            false,
            false,
        ),
        create_account_txn(
            sender.account(),
            &Account::new(),
            Some(11),
            executor.get_block_time_seconds(),
            false,
            false,
        ),
    ];
    let output = &executor
        .execute_block(txns)
        .expect("The VM should not fail to startup");
    output[1].gas_used()
});

/// The gas cost of a create-account transaction where the receiver already exists.
///
/// This includes the cost of the event counter creation. As such the cost of the transaction
/// would be higher and the balance required must be higher.
/// All such transactions are expected to cost the same gas.
pub static CREATE_EXISTING_ACCOUNT_FIRST: Lazy<u64> = Lazy::new(|| {
    let mut executor = FakeExecutor::from_head_genesis();
    let sender = AccountData::new(1_000_000, Some(10));
    let receiver = AccountData::new(1_000_000, Some(10));
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    let txn = create_account_txn(
        sender.account(),
        receiver.account(),
        Some(10),
        executor.get_block_time_seconds(),
        false,
        false,
    );
    compute_gas_used(txn, &mut executor)
});

/// The gas cost of a create-account transaction where the receiver already exists.
///
/// This is the cost after the event counter has been created.
/// All such transactions are expected to cost the same gas.
pub static CREATE_EXISTING_ACCOUNT_NEXT: Lazy<u64> = Lazy::new(|| {
    let mut executor = FakeExecutor::from_head_genesis();
    let sender = AccountData::new(1_000_000, Some(10));
    let receiver = AccountData::new(1_000_000, Some(10));
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    let txns = vec![
        create_account_txn(
            sender.account(),
            &Account::new(),
            Some(10),
            executor.get_block_time_seconds(),
            false,
            false,
        ),
        create_account_txn(
            sender.account(),
            receiver.account(),
            Some(11),
            executor.get_block_time_seconds(),
            false,
            false,
        ),
    ];
    let output = &executor
        .execute_block(txns)
        .expect("The VM should not fail to startup");
    output[1].gas_used()
});

/// The gas cost of a peer-to-peer transaction.
///
/// All such transactions are expected to cost the same gas.
pub static PEER_TO_PEER: Lazy<u64> = Lazy::new(|| {
    // Compute gas used by running a placeholder transaction.
    let mut executor = FakeExecutor::from_head_genesis();
    let sender = AccountData::new(1_000_000, Some(10));
    let receiver = AccountData::new(1_000_000, Some(10));
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    let txn = peer_to_peer_txn(
        sender.account(),
        receiver.account(),
        Some(10),
        20_000,
        0,
        executor.get_block_time_seconds(),
        false,
        false,
    );
    compute_gas_used(txn, &mut executor)
});

/// The gas cost of a peer-to-peer transaction with an insufficient balance.
///
/// All such transactions are expected to cost the same gas.
pub static PEER_TO_PEER_TOO_LOW: Lazy<u64> = Lazy::new(|| {
    let mut executor = FakeExecutor::from_head_genesis();
    // The gas amount is the minimum that needs to be reserved, so use a value that's clearly
    // higher than that.
    let balance = TXN_RESERVED + 10_000;
    let sender = AccountData::new(balance, Some(10));
    let receiver = AccountData::new(1_000_000, Some(10));
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);

    let txn = peer_to_peer_txn(
        sender.account(),
        receiver.account(),
        Some(10),
        balance + 1,
        0,
        executor.get_block_time_seconds(),
        false,
        false,
    );
    compute_gas_used(txn, &mut executor)
});

/// The gas cost of a peer-to-peer transaction that creates a new account.
///
/// This includes the cost of the event counter creation. As such the cost of the transaction
/// would be higher and the balance required must be higher.
/// All such transactions are expected to cost the same gas.
pub static PEER_TO_PEER_NEW_RECEIVER_FIRST: Lazy<u64> = Lazy::new(|| {
    // Compute gas used by running a placeholder transaction.
    let mut executor = FakeExecutor::from_head_genesis();
    let sender = AccountData::new(1_000_000, Some(10));
    executor.add_account_data(&sender);
    let receiver = Account::new();

    let txn = peer_to_peer_txn(
        sender.account(),
        &receiver,
        Some(10),
        20_000,
        0,
        executor.get_block_time_seconds(),
        false,
        false,
    );
    compute_gas_used(txn, &mut executor)
});

/// The gas cost of a peer-to-peer transaction that creates a new account.
///
/// This is the cost after the event counter has been created.
/// All such transactions are expected to cost the same gas.
pub static PEER_TO_PEER_NEW_RECEIVER_NEXT: Lazy<u64> = Lazy::new(|| {
    // Compute gas used by running a placeholder transaction.
    let mut executor = FakeExecutor::from_head_genesis();
    let sender = AccountData::new(1_000_000, Some(10));
    executor.add_account_data(&sender);

    let txns = vec![
        peer_to_peer_txn(
            sender.account(),
            &Account::new(),
            Some(10),
            20_000,
            0,
            executor.get_block_time_seconds(),
            false,
            false,
        ),
        peer_to_peer_txn(
            sender.account(),
            &Account::new(),
            Some(11),
            20_000,
            0,
            executor.get_block_time_seconds(),
            false,
            false,
        ),
    ];
    let output = &executor
        .execute_block(txns)
        .expect("The VM should not fail to startup");
    output[1].gas_used()
});

/// The gas cost of a peer-to-peer transaction that tries to create a new account, but fails
/// because of an insufficient balance.
///
/// This includes the cost of the event counter creation. As such the cost of the transaction
/// would be higher and the balance required must be higher.
/// All such transactions are expected to cost the same gas.
pub static PEER_TO_PEER_NEW_RECEIVER_TOO_LOW_FIRST: Lazy<u64> = Lazy::new(|| {
    let mut executor = FakeExecutor::from_head_genesis();
    // The gas amount is the minimum that needs to be reserved, so use a value that's
    // clearly higher than that.
    let balance = TXN_RESERVED + 10_000;
    let sender = AccountData::new(balance, Some(10));
    executor.add_account_data(&sender);
    let receiver = Account::new();

    let txn = peer_to_peer_txn(
        sender.account(),
        &receiver,
        Some(10),
        balance + 1,
        0,
        executor.get_block_time_seconds(),
        false,
        false,
    );
    compute_gas_used(txn, &mut executor)
});

/// The gas cost of a peer-to-peer transaction that tries to create a new account, but fails
/// because of an insufficient balance.
///
/// This is the cost after the event counter has been created.
/// All such transactions are expected to cost the same gas.
pub static PEER_TO_PEER_NEW_RECEIVER_TOO_LOW_NEXT: Lazy<u64> = Lazy::new(|| {
    let mut executor = FakeExecutor::from_head_genesis();
    // The gas amount is the minimum that needs to be reserved, so use a value that's
    // clearly higher than that.
    let balance = (2 * TXN_RESERVED) + 20_000;
    let sender = AccountData::new(balance, Some(10));
    executor.add_account_data(&sender);

    let txns = vec![
        peer_to_peer_txn(
            sender.account(),
            &Account::new(),
            Some(10),
            10_000,
            0,
            executor.get_block_time_seconds(),
            false,
            false,
        ),
        peer_to_peer_txn(
            sender.account(),
            &Account::new(),
            Some(11),
            balance,
            0,
            executor.get_block_time_seconds(),
            false,
            false,
        ),
    ];
    let output = &executor
        .execute_block(txns)
        .expect("The VM should not fail to startup");
    output[1].gas_used()
});

fn compute_gas_used(txn: SignedTransaction, executor: &mut FakeExecutor) -> u64 {
    let output = &executor.execute_transaction(txn);
    output.gas_used()
}
