// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::{
    common_transactions::peer_to_peer_txn,
    data_store::{FakeDataStore, GENESIS_CHANGE_SET_HEAD},
    executor::FakeExecutor,
};
use aptos_types::{
    state_store::TStateView,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, ChangeSet, Transaction,
        TransactionStatus, WriteSetPayload,
    },
    write_set::TransactionWrite,
};
use aptos_vm::{data_cache::AsMoveResolver, AptosVM};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use claims::{assert_err, assert_ok, assert_some};
use move_core_types::vm_status::StatusCode;

fn create_simple_user_transaction(executor: &mut FakeExecutor) -> Transaction {
    let sender = executor.create_raw_account_data(1_000_000, 10);
    let receiver = executor.create_raw_account_data(100_000, 10);
    Transaction::UserTransaction(peer_to_peer_txn(
        sender.account(),
        receiver.account(),
        11,
        1000,
        0,
    ))
}

#[test]
fn no_deletion_in_genesis() {
    let genesis = GENESIS_CHANGE_SET_HEAD.clone();
    assert!(!genesis.write_set().iter().any(|(_, op)| op.is_deletion()))
}

#[test]
fn execute_genesis_write_set() {
    use Transaction::*;

    let executor = FakeExecutor::no_genesis();
    println!("{:#?}", *GENESIS_CHANGE_SET_HEAD);
    let txn = GenesisTransaction(WriteSetPayload::Direct(GENESIS_CHANGE_SET_HEAD.clone()));

    let mut output = assert_ok!(executor.execute_transaction_block(vec![txn]));

    // Executing the genesis transaction should succeed.
    assert_eq!(output.len(), 1);
    assert!(!output.pop().unwrap().status().is_discarded())
}

#[test]
fn execute_genesis_and_drop_other_transaction() {
    use Transaction::*;

    let mut executor = FakeExecutor::no_genesis();
    let txn1 = GenesisTransaction(WriteSetPayload::Direct(GENESIS_CHANGE_SET_HEAD.clone()));
    let txn2 = create_simple_user_transaction(&mut executor);

    let mut output = assert_ok!(executor.execute_transaction_block(vec![txn1, txn2]));

    // Transaction that comes after genesis should be dropped.
    assert_eq!(output.len(), 2);
    assert_eq!(output.pop().unwrap().status(), &TransactionStatus::Retry)
}

#[test]
fn fail_no_epoch_change_write_set() {
    use Transaction::*;

    let mut executor = FakeExecutor::no_genesis();
    let txn1 = GenesisTransaction(WriteSetPayload::Direct(ChangeSet::empty()));
    let txn2 = create_simple_user_transaction(&mut executor);

    let err = assert_err!(executor.execute_transaction_block(vec![txn1, txn2]));
    assert_eq!(err.status_code(), StatusCode::INVALID_WRITE_SET);
}

#[test]
fn vm_cannot_execute_direct_write_set_payload() {
    use SignatureVerifiedTransaction::*;
    use Transaction::*;

    let state_view = FakeDataStore::default();
    let vm = AptosVM::new(&state_view);

    let txn = Valid(GenesisTransaction(WriteSetPayload::Direct(
        ChangeSet::empty(),
    )));

    let log_context = AdapterLogSchema::new(state_view.id(), 0);
    let resolver = state_view.as_move_resolver();
    let (vm_status, output) =
        assert_ok!(vm.execute_single_transaction(&txn, &resolver, &log_context));

    // Genesis direct write set payload cannot be executed as a single transaction.
    assert!(output.status().is_discarded());
    assert_eq!(vm_status.status_code(), StatusCode::FEATURE_UNDER_GATING);
    assert_eq!(
        assert_some!(vm_status.message()).as_str(),
        "Direct write set payload cannot be executed directly"
    )
}
