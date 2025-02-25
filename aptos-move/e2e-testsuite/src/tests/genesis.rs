// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::{
    common_transactions::peer_to_peer_txn, data_store::GENESIS_CHANGE_SET_HEAD,
    executor::FakeExecutor, feature_flags_for_orderless,
};
use aptos_types::{
    transaction::{ChangeSet, Transaction, TransactionStatus, WriteSetPayload},
    write_set::TransactionWrite,
};
use move_core_types::vm_status::StatusCode;
use rstest::rstest;

#[test]
fn no_deletion_in_genesis() {
    let genesis = GENESIS_CHANGE_SET_HEAD.clone();
    assert!(!genesis.write_set().iter().any(|(_, op)| op.is_deletion()))
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

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn execute_genesis_and_drop_other_transaction(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut executor = FakeExecutor::no_genesis();
    executor.enable_features(feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions), vec![]);
    let txn =
        Transaction::GenesisTransaction(WriteSetPayload::Direct(GENESIS_CHANGE_SET_HEAD.clone()));

    let sender = executor.create_raw_account_data(1_000_000, if stateless_account { None } else { Some(10) });
    let receiver = executor.create_raw_account_data(100_000, Some(10));
    let txn2 = peer_to_peer_txn(sender.account(), receiver.account(), Some(11), 1000, 0, use_txn_payload_v2_format, use_orderless_transactions);

    let mut output = executor
        .execute_transaction_block(vec![txn, Transaction::UserTransaction(txn2)])
        .unwrap();

    // Transaction that comes after genesis should be dropped.
    assert_eq!(output.len(), 2);
    assert_eq!(output.pop().unwrap().status(), &TransactionStatus::Retry)
}

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn fail_no_epoch_change_write_set(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut executor = FakeExecutor::no_genesis();
    executor.enable_features(feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions), vec![]);
    let txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(ChangeSet::empty()));

    let sender = executor.create_raw_account_data(1_000_000, if stateless_account { None } else { Some(10) });
    let receiver = executor.create_raw_account_data(100_000, Some(10));
    let txn2 = peer_to_peer_txn(sender.account(), receiver.account(), Some(11), 1000, 0, use_txn_payload_v2_format, use_orderless_transactions);

    let output_err = executor
        .execute_transaction_block(vec![txn, Transaction::UserTransaction(txn2)])
        .unwrap_err();
    assert_eq!(StatusCode::INVALID_WRITE_SET, output_err.status_code());
}
