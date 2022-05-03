// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{pruner::*, AptosDB, ChangeSet, LedgerStore, TransactionStore};
use aptos_temppath::TempPath;
use proptest::proptest;

use aptos_types::{
    account_address::AccountAddress,
    block_metadata::BlockMetadata,
    transaction::{SignedTransaction, Transaction},
};

use aptos_types::{
    transaction::{TransactionInfo, Version},
    write_set::WriteSet,
};
use proptest::{collection::vec, prelude::*};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_txn_store_pruner(txns in vec(
        prop_oneof![
            any::<BlockMetadata>().prop_map(Transaction::BlockMetadata),
            any::<SignedTransaction>().prop_map(Transaction::UserTransaction),
        ], 1..100,),
        txn_infos in vec(any::<TransactionInfo>(),100,),
        step_size in 1usize..20,
    ) {
        verify_txn_store_pruner(txns, txn_infos, step_size)
    }

     #[test]
    fn test_write_set_pruner(
        write_set in vec(any::<WriteSet>(), 100),
        ) {
            verify_write_set_pruner(write_set);
        }
}

fn verify_write_set_pruner(write_sets: Vec<WriteSet>) {
    let tmp_dir = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp_dir);
    let transaction_store = &aptos_db.transaction_store;
    let num_write_sets = write_sets.len();

    let pruner = Pruner::new(
        Arc::clone(&aptos_db.db),
        StoragePrunerConfig {
            state_store_prune_window: Some(0),
            ledger_prune_window: Some(0),
            pruning_batch_size: 1,
        },
        Arc::clone(transaction_store),
        Arc::clone(&aptos_db.ledger_store),
        Arc::clone(&aptos_db.event_store),
    );

    // write sets
    let mut cs = ChangeSet::new();
    for (ver, ws) in write_sets.iter().enumerate() {
        transaction_store
            .put_write_set(ver as Version, ws, &mut cs)
            .unwrap();
    }
    aptos_db.db.write_schemas(cs.batch).unwrap();
    // start pruning write sets in batches of size 2 and verify transactions have been pruned from DB
    for i in (0..=num_write_sets).step_by(2) {
        pruner
            .wake_and_wait(
                i as u64, /* latest_version */
                PrunerIndex::LedgerPrunerIndex as usize,
            )
            .unwrap();
        // ensure that all transaction up to i * 2 has been pruned
        for j in 0..i {
            assert!(transaction_store.get_write_set(j as u64).is_err());
        }
        // ensure all other are valid in DB
        for j in i..num_write_sets {
            let write_set_from_db = transaction_store.get_write_set(j as u64).unwrap();
            assert_eq!(write_set_from_db, *write_sets.get(j).unwrap());
        }
    }
}

fn verify_txn_store_pruner(
    txns: Vec<Transaction>,
    txn_infos: Vec<TransactionInfo>,
    step_size: usize,
) {
    let tmp_dir = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp_dir);
    let transaction_store = &aptos_db.transaction_store;
    let ledger_store = LedgerStore::new(Arc::clone(&aptos_db.db));
    let num_transaction = txns.len();

    let pruner = Pruner::new(
        Arc::clone(&aptos_db.db),
        StoragePrunerConfig {
            state_store_prune_window: Some(0),
            ledger_prune_window: Some(0),
            pruning_batch_size: 1,
        },
        Arc::clone(transaction_store),
        Arc::clone(&aptos_db.ledger_store),
        Arc::clone(&aptos_db.event_store),
    );

    let ledger_version = num_transaction as Version - 1;
    put_txn_in_store(
        &aptos_db,
        transaction_store,
        &ledger_store,
        &txn_infos,
        &txns,
    );

    // start pruning transactions batches of size step_size and verify transactions have been pruned from DB
    for i in (0..=num_transaction).step_by(step_size) {
        pruner
            .wake_and_wait(
                i as u64, /* latest_version */
                PrunerIndex::LedgerPrunerIndex as usize,
            )
            .unwrap();
        // ensure that all transaction up to i * 2 has been pruned
        assert_eq!(*pruner.last_version_sent_to_pruners.lock(), i as u64);
        for j in 0..i {
            verify_txn_not_in_store(transaction_store, &txns, j as u64, ledger_version);
        }
        // ensure all other are valid in DB
        for j in i..num_transaction {
            verify_txn_in_store(
                transaction_store,
                &ledger_store,
                &txns,
                j as u64,
                ledger_version,
            );
        }
    }
}

fn verify_txn_not_in_store(
    transaction_store: &TransactionStore,
    txns: &[Transaction],
    index: u64,
    ledger_version: u64,
) {
    // Ensure that all transaction from transaction schema store has been pruned
    assert!(transaction_store.get_transaction(index).is_err());
    // Ensure that transaction by account store has been pruned
    if let Transaction::UserTransaction(txn) = txns.get(index as usize).unwrap() {
        assert!(transaction_store
            .get_account_transaction_version(txn.sender(), txn.sequence_number(), ledger_version,)
            .unwrap()
            .is_none());
    }
}

fn verify_txn_in_store(
    transaction_store: &TransactionStore,
    ledger_store: &LedgerStore,
    txns: &[Transaction],
    index: u64,
    ledger_version: u64,
) {
    verify_transaction_in_transaction_store(
        transaction_store,
        txns.get(index as usize).unwrap(),
        index as u64,
    );
    if let Transaction::UserTransaction(txn) = txns.get(index as usize).unwrap() {
        verify_transaction_in_account_txn_by_version_index(
            transaction_store,
            index as u64,
            txn.sender(),
            txn.sequence_number(),
            ledger_version,
        );
    }
    // Ensure that transaction accumulator is in DB. This can be done by trying
    // to read transaction proof
    assert!(ledger_store
        .get_transaction_proof(index, ledger_version)
        .is_ok());
}

fn put_txn_in_store(
    aptos_db: &AptosDB,
    transaction_store: &TransactionStore,
    ledger_store: &LedgerStore,
    txn_infos: &[TransactionInfo],
    txns: &[Transaction],
) {
    let mut cs = ChangeSet::new();
    for i in 0..txns.len() {
        transaction_store
            .put_transaction(i as u64, txns.get(i).unwrap(), &mut cs)
            .unwrap();
    }
    ledger_store
        .put_transaction_infos(0, txn_infos, &mut cs)
        .unwrap();
    aptos_db.db.write_schemas(cs.batch).unwrap();
}

fn verify_transaction_in_transaction_store(
    transaction_store: &TransactionStore,
    expected_value: &Transaction,
    version: Version,
) {
    let txn = transaction_store.get_transaction(version).unwrap();
    assert_eq!(txn, *expected_value)
}

fn verify_transaction_in_account_txn_by_version_index(
    transaction_store: &TransactionStore,
    expected_value: Version,
    address: AccountAddress,
    sequence_number: u64,
    ledger_version: Version,
) {
    let transaction = transaction_store
        .get_account_transaction_version(address, sequence_number, ledger_version)
        .unwrap()
        .unwrap();
    assert_eq!(transaction, expected_value)
}
