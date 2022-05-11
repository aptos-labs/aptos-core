// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use aptos_crypto::HashValue;
use aptos_temppath::TempPath;
use aptos_types::state_store::{state_key::StateKey, state_value::StateValue};

use crate::{change_set::ChangeSet, pruner::*, state_store::StateStore, AptosDB};

fn put_value_set(
    db: &DB,
    state_store: &StateStore,
    value_set: Vec<(StateKey, StateValue)>,
    version: Version,
) -> HashValue {
    let mut cs = ChangeSet::new();
    let value_set: HashMap<_, _> = value_set
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();

    let root = state_store
        .put_value_sets(vec![&value_set], None, version, &mut cs)
        .unwrap()[0];
    db.write_schemas(cs.batch).unwrap();
    state_store.set_latest_version(version);

    root
}

fn verify_state_in_store(
    state_store: &StateStore,
    key: StateKey,
    expected_value: Option<&StateValue>,
    version: Version,
) {
    let (value, _proof) = state_store
        .get_value_with_proof_by_version(&key, version)
        .unwrap();

    assert_eq!(value.as_ref(), expected_value);
}

#[test]
fn test_state_store_pruner() {
    let key = StateKey::Raw(String::from("test_key1").into_bytes());

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
        let value = StateValue::from(vec![i as u8]);
        root_hashes.push(put_value_set(
            &db,
            state_store,
            vec![(key.clone(), value.clone())],
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
                key.clone(),
                Some(&StateValue::from(vec![i as u8])),
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
                .get_value_with_proof_by_version(&key, i as u64)
                .is_err());
        }
        for i in prune_batch_size..num_versions as usize {
            verify_state_in_store(
                state_store,
                key.clone(),
                Some(&StateValue::from(vec![i as u8])),
                i as u64,
            );
        }
    }
}

#[test]
fn test_worker_quit_eagerly() {
    let key = StateKey::Raw(String::from("test_key1").into_bytes());

    let value0 = StateValue::from(String::from("test_val1").into_bytes());
    let value1 = StateValue::from(String::from("test_val2").into_bytes());
    let value2 = StateValue::from(String::from("test_val3").into_bytes());

    let tmp_dir = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp_dir);
    let db = aptos_db.db;
    let state_store = &StateStore::new(Arc::clone(&db));

    let _root0 = put_value_set(
        &db,
        state_store,
        vec![(key.clone(), value0.clone())],
        0, /* version */
    );
    let _root1 = put_value_set(
        &db,
        state_store,
        vec![(key.clone(), value1.clone())],
        1, /* version */
    );
    let _root2 = put_value_set(
        &db,
        state_store,
        vec![(key.clone(), value2.clone())],
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
        verify_state_in_store(state_store, key.clone(), Some(&value0), 0);
        verify_state_in_store(state_store, key.clone(), Some(&value1), 1);
        verify_state_in_store(state_store, key, Some(&value2), 2);
    }
}
