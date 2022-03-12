// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{pruner::*, AptosDB, ChangeSet, TransactionStore};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519Signature},
    PrivateKey, Uniform,
};
use aptos_temppath::TempPath;

use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    transaction::{RawTransaction, Script, SignedTransaction, Transaction, TransactionPayload},
};

/// Creates a single test transaction
fn create_test_transaction(sequence_number: u64) -> Transaction {
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();

    let transaction_payload = TransactionPayload::Script(Script::new(vec![], vec![], vec![]));
    let raw_transaction = RawTransaction::new(
        AccountAddress::random(),
        sequence_number,
        transaction_payload,
        0,
        0,
        "".into(),
        0,
        ChainId::new(10),
    );
    let signed_transaction = SignedTransaction::new(
        raw_transaction,
        public_key,
        Ed25519Signature::dummy_signature(),
    );

    Transaction::UserTransaction(signed_transaction)
}

fn verify_transaction_in_store(
    transaction_store: &TransactionStore,
    expected_value: &Transaction,
    version: Version,
) {
    let txn = transaction_store.get_transaction(version).unwrap();
    assert_eq!(txn, *expected_value)
}

#[test]
fn test_txn_store_pruner() {
    let tmp_dir = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp_dir);
    let transaction_store = &aptos_db.transaction_store;
    let num_transaction = 50;
    let mut transactions: Vec<Transaction> = vec![];

    let pruner = Pruner::new(
        Arc::clone(&aptos_db.db),
        StoragePrunerConfig {
            state_store_prune_window: Some(0),
            default_prune_window: Some(0),
        }, /* historical_versions_to_keep */
    );

    for i in 0..num_transaction {
        let transaction = create_test_transaction(i);
        transactions.push(transaction.clone());
        let mut cs = ChangeSet::new();
        transaction_store
            .put_transaction(i, &transaction, &mut cs)
            .unwrap();
        aptos_db.db.write_schemas(cs.batch).unwrap();
    }

    // start pruning transaction in the batch of 2 and verify transactions have been pruned from DB
    for i in (0..=num_transaction).step_by(2) {
        pruner
            .wake_and_wait(i /* latest_version */, TRANSACTION_STORE_PRUNER_INDEX)
            .unwrap();
        // ensure that all transaction up to i * 2 has been pruned
        for j in 0..i {
            assert!(transaction_store.get_transaction(j).is_err());
        }
        // ensure all other are valid in DB
        for j in i..num_transaction {
            verify_transaction_in_store(
                transaction_store,
                transactions.get(j as usize).unwrap(),
                j,
            );
        }
    }
}
