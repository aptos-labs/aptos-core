// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{change_set::ChangeSet, pruner::*, state_store::StateStore, AptosDB};
use aptos_crypto::HashValue;
use aptos_temppath::TempPath;
use aptos_types::{account_address::AccountAddress, account_state_blob::AccountStateBlob};
use std::collections::HashMap;

fn put_account_state_set(
    db: &DB,
    state_store: &StateStore,
    account_state_set: Vec<(AccountAddress, AccountStateBlob)>,
    version: Version,
) -> HashValue {
    let mut cs = ChangeSet::new();
    let root = state_store
        .put_account_state_sets(
            vec![account_state_set.into_iter().collect::<HashMap<_, _>>()],
            None,
            version,
            &mut cs,
        )
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
        .get_account_state_with_proof_by_version(address, version)
        .unwrap();
    assert_eq!(value.as_ref(), expected_value);
}

#[test]
fn test_state_store_pruner() {
    let address = AccountAddress::new([1u8; AccountAddress::LENGTH]);
    let value0 = AccountStateBlob::from(vec![0x01]);
    let value1 = AccountStateBlob::from(vec![0x02]);
    let value2 = AccountStateBlob::from(vec![0x03]);

    let tmp_dir = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp_dir);
    let db = aptos_db.db;
    let state_store = &StateStore::new(Arc::clone(&db), true /* account_count_migration */);
    let transaction_store = &aptos_db.transaction_store;
    let pruner = Pruner::new(
        Arc::clone(&db),
        StoragePrunerConfig {
            state_store_prune_window: Some(0),
            default_prune_window: Some(0),
        },
        Arc::clone(transaction_store),
        Arc::clone(&aptos_db.ledger_store),
        Arc::clone(&aptos_db.event_store),
    );

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

    // Prune till version=0.
    {
        pruner
            .wake_and_wait(
                0, /* latest_version */
                PrunerIndex::StateStorePrunerIndex as usize,
            )
            .unwrap();
        verify_state_in_store(state_store, address, Some(&value0), 0);
        verify_state_in_store(state_store, address, Some(&value1), 1);
        verify_state_in_store(state_store, address, Some(&value2), 2);
    }
    // Prune till version=1.
    {
        pruner
            .wake_and_wait(
                1, /* latest_version */
                PrunerIndex::StateStorePrunerIndex as usize,
            )
            .unwrap();
        // root0 is gone.
        assert!(state_store
            .get_account_state_with_proof_by_version(address, 0)
            .is_err());
        // root1 is still there.
        verify_state_in_store(state_store, address, Some(&value1), 1);
        verify_state_in_store(state_store, address, Some(&value2), 2);
    }
    // Prune till version=2.
    {
        pruner
            .wake_and_wait(
                2, /* latest_version */
                PrunerIndex::StateStorePrunerIndex as usize,
            )
            .unwrap();
        // root1 is gone.
        assert!(state_store
            .get_account_state_with_proof_by_version(address, 1)
            .is_err());
        // root2 is still there.
        verify_state_in_store(state_store, address, Some(&value2), 2);
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
    let state_store = &StateStore::new(Arc::clone(&db), true /* account_count_migration */);

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
        );
        command_sender
            .send(Command::Prune {
                target_db_versions: vec![1, 0, 0, 0, 0, 0],
            })
            .unwrap();
        command_sender
            .send(Command::Prune {
                target_db_versions: vec![2, 0, 0, 0, 0, 0],
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
