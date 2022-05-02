// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{change_set::ChangeSet, pruner::*, state_store::StateStore, AptosDB};
use aptos_crypto::HashValue;
use aptos_temppath::TempPath;
use aptos_types::{
    account_address::AccountAddress,
    account_state_blob::AccountStateBlob,
    state_store::{state_key::StateKey, state_value::StateValue},
};
use std::{collections::HashMap, convert::TryFrom};

fn put_account_state_set(
    db: &DB,
    state_store: &StateStore,
    account_state_set: Vec<(AccountAddress, AccountStateBlob)>,
    version: Version,
) -> HashValue {
    let mut cs = ChangeSet::new();
    let value_state_set: HashMap<_, _> = account_state_set
        .iter()
        .map(|(address, blob)| {
            (
                StateKey::AccountAddressKey(*address),
                StateValue::from(blob.clone()),
            )
        })
        .collect();
    let root = state_store
        .put_value_sets(vec![&value_state_set], None, version, &mut cs)
        .unwrap()[0];
    db.write_schemas(cs.batch).unwrap();

    root
}

fn verify_state_in_store(
    state_store: &StateStore,
    address: AccountAddress,
    expected_value: Option<&AccountStateBlob>,
    version: Version,
) {
    let (value, _proof) = state_store
        .get_value_with_proof_by_version(&StateKey::AccountAddressKey(address), version)
        .unwrap();

    assert_eq!(
        &AccountStateBlob::try_from(value.unwrap()).unwrap(),
        expected_value.unwrap()
    );
}

#[test]
fn test_state_store_pruner() {
    let address = AccountAddress::new([1u8; AccountAddress::LENGTH]);
    let prune_batch_size = 10;
    let num_versions = 25;
    let tmp_dir = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp_dir);
    let db = aptos_db.db;
    let state_store = &StateStore::new(Arc::clone(&db));
    let transaction_store = &aptos_db.transaction_store;
    let pruner = Pruner::new(
        Arc::clone(&db),
        StoragePrunerConfig {
            state_store_prune_window: Some(0),
            ledger_prune_window: Some(0),
            pruning_batch_size: prune_batch_size,
        },
        Arc::clone(transaction_store),
        Arc::clone(&aptos_db.ledger_store),
        Arc::clone(&aptos_db.event_store),
    );

    let mut root_hashes = vec![];
    // Insert 25 values in the db.
    for i in 0..num_versions {
        let value = AccountStateBlob::from(vec![i as u8]);
        root_hashes.push(put_account_state_set(
            &db,
            state_store,
            vec![(address, value.clone())],
            i as u64, /* version */
        ));
    }

    // Prune till version=0. This should basically be a no-op
    {
        pruner
            .wake_and_wait(
                0, /* latest_version */
                PrunerIndex::StateStorePrunerIndex as usize,
            )
            .unwrap();
        for i in 0..num_versions {
            verify_state_in_store(
                state_store,
                address,
                Some(&AccountStateBlob::from(vec![i as u8])),
                i,
            );
        }
    }

    // Test for batched pruning, since we use a batch size of 10, updating the latest version to
    // less than 10 should not perform any actual pruning.
    assert!(pruner
        .wake_and_wait(
            5, /* latest_version */
            PrunerIndex::StateStorePrunerIndex as usize,
        )
        .is_err());

    // Notify the pruner to update the version to be 10 - since we use a batch size of 10,
    // we expect versions 0 to 9 to be pruned.
    {
        pruner
            .wake_and_wait(
                prune_batch_size as u64, /* latest_version */
                PrunerIndex::StateStorePrunerIndex as usize,
            )
            .unwrap();
        for i in 0..prune_batch_size {
            assert!(state_store
                .get_value_with_proof_by_version(&StateKey::AccountAddressKey(address), i as u64)
                .is_err());
        }
        for i in prune_batch_size..num_versions as usize {
            verify_state_in_store(
                state_store,
                address,
                Some(&AccountStateBlob::from(vec![i as u8])),
                i as u64,
            );
        }
    }
}

#[test]
fn test_worker_quit_eagerly() {
    let address = AccountAddress::new([1u8; AccountAddress::LENGTH]);
    let value0 = AccountStateBlob::from(vec![0x01]);
    let value1 = AccountStateBlob::from(vec![0x02]);
    let value2 = AccountStateBlob::from(vec![0x03]);

    let tmp_dir = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp_dir);
    let db = aptos_db.db;
    let state_store = &StateStore::new(Arc::clone(&db));

    let _root0 = put_account_state_set(
        &db,
        state_store,
        vec![(address, value0.clone())],
        0, /* version */
    );
    let _root1 = put_account_state_set(
        &db,
        state_store,
        vec![(address, value1.clone())],
        1, /* version */
    );
    let _root2 = put_account_state_set(
        &db,
        state_store,
        vec![(address, value2.clone())],
        2, /* version */
    );

    {
        let (command_sender, command_receiver) = channel();
        let worker = Worker::new(
            Arc::clone(&db),
            Arc::clone(&aptos_db.transaction_store),
            Arc::clone(&aptos_db.ledger_store),
            Arc::clone(&aptos_db.event_store),
            command_receiver,
            Arc::new(Mutex::new(vec![0, 0])), /* progress */
            100,
        );
        command_sender
            .send(Command::Prune {
                target_db_versions: vec![1, 0],
            })
            .unwrap();
        command_sender
            .send(Command::Prune {
                target_db_versions: vec![2, 0],
            })
            .unwrap();
        command_sender.send(Command::Quit).unwrap();
        // Worker quits immediately although `Command::Quit` is not the first command sent.
        worker.work();
        verify_state_in_store(state_store, address, Some(&value0), 0);
        verify_state_in_store(state_store, address, Some(&value1), 1);
        verify_state_in_store(state_store, address, Some(&value2), 2);
    }
}
