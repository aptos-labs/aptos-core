// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::{common_transactions::peer_to_peer_txn, executor::FakeExecutor};
use aptos_transaction_simulation::GENESIS_CHANGE_SET_HEAD;
use aptos_types::{
    transaction::{ChangeSet, Transaction, TransactionStatus, WriteSetPayload},
    write_set::TransactionWrite,
};
use move_core_types::vm_status::StatusCode;

#[test]
fn no_deletion_in_genesis() {
    let genesis = GENESIS_CHANGE_SET_HEAD.clone();
    assert!(!genesis
        .write_set()
        .expect_v0()
        .iter()
        .any(|(_, op)| op.is_deletion()))
}

#[test]
fn execute_genesis_write_set() {
    let executor = FakeExecutor::no_genesis();
    println!("{:#?}", *GENESIS_CHANGE_SET_HEAD);
    let txn =
        Transaction::GenesisTransaction(WriteSetPayload::Direct(GENESIS_CHANGE_SET_HEAD.clone()));
    let mut output = executor.execute_transaction_block(vec![txn]).unwrap();

    // Executing the genesis transaction should succeed
    assert_eq!(output.len(), 1);
    assert!(!output.pop().unwrap().status().is_discarded())
}

#[test]
fn execute_genesis_and_drop_other_transaction() {
    let mut executor = FakeExecutor::no_genesis();
    let txn =
        Transaction::GenesisTransaction(WriteSetPayload::Direct(GENESIS_CHANGE_SET_HEAD.clone()));

    let sender = executor.create_raw_account_data(1_000_000, 10);
    let receiver = executor.create_raw_account_data(100_000, 10);
    let txn2 = peer_to_peer_txn(sender.account(), receiver.account(), 11, 1000, 0);

    let mut output = executor
        .execute_transaction_block(vec![txn, Transaction::UserTransaction(txn2)])
        .unwrap();

    // Transaction that comes after genesis should be dropped.
    assert_eq!(output.len(), 2);
    assert_eq!(output.pop().unwrap().status(), &TransactionStatus::Retry)
}

#[test]
fn fail_no_epoch_change_write_set() {
    let mut executor = FakeExecutor::no_genesis();
    let txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(ChangeSet::empty()));

    let sender = executor.create_raw_account_data(1_000_000, 10);
    let receiver = executor.create_raw_account_data(100_000, 10);
    let txn2 = peer_to_peer_txn(sender.account(), receiver.account(), 11, 1000, 0);

    let output_err = executor
        .execute_transaction_block(vec![txn, Transaction::UserTransaction(txn2)])
        .unwrap_err();
    assert_eq!(StatusCode::INVALID_WRITE_SET, output_err.status_code());
}
