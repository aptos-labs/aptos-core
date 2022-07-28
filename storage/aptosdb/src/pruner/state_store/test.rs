// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use aptos_crypto::HashValue;
use aptos_temppath::TempPath;
use aptos_types::state_store::{state_key::StateKey, state_value::StateValue};
use schemadb::ReadOptions;
use storage_interface::{jmt_update_refs, jmt_updates, DbReader};

use crate::stale_node_index::StaleNodeIndexSchema;
use crate::{change_set::ChangeSet, pruner::*, state_store::StateStore, AptosDB};

fn put_value_set(
    db: &DB,
    state_store: &StateStore,
    value_set: Vec<(StateKey, StateValue)>,
    version: Version,
) -> HashValue {
    let value_set: HashMap<_, _> = value_set
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let jmt_updates = jmt_updates(&value_set);

    let root = state_store
        .merklize_value_set(
            jmt_update_refs(&jmt_updates),
            None,
            version,
            version.checked_sub(1),
        )
        .unwrap();

    let mut cs = ChangeSet::new();
    state_store
        .put_value_sets(vec![&value_set], version, &mut cs)
        .unwrap();
    db.write_schemas(cs.batch).unwrap();

    root
}

fn verify_state_in_store(
    state_store: &StateStore,
    key: StateKey,
    expected_value: Option<&StateValue>,
    version: Version,
) {
    let (value, _proof) = state_store
        .get_state_value_with_proof_by_version(&key, version)
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
    let state_store = &StateStore::new(
        Arc::clone(&aptos_db.ledger_db),
        Arc::clone(&aptos_db.state_merkle_db),
        1000,  /* snapshot_size_threshold, does not matter */
        false, /* hack_for_tests */
    );
    let pruner = Pruner::new(
        Arc::clone(&aptos_db.ledger_db),
        Arc::clone(&aptos_db.state_merkle_db),
        StoragePrunerConfig {
            state_store_prune_window: Some(0),
            ledger_prune_window: Some(0),
            ledger_pruning_batch_size: prune_batch_size,
            state_store_pruning_batch_size: prune_batch_size,
        },
    );

    let mut root_hashes = vec![];
    // Insert 25 values in the db.
    for i in 0..num_versions {
        let value = StateValue::from(vec![i as u8]);
        root_hashes.push(put_value_set(
            &aptos_db.ledger_db,
            state_store,
            vec![(key.clone(), value.clone())],
            i as u64, /* version */
        ));
    }

    // Prune till version=0. This should basically be a no-op
    {
        pruner
            .wake_and_wait_state_pruner(0 /* latest_version */)
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

    // Notify the pruner to update the version to be 10 - since we use a batch size of 10,
    // we expect versions 0 to 9 to be pruned.
    {
        pruner
            .wake_and_wait_state_pruner(prune_batch_size as u64 /* latest_version */)
            .unwrap();
        for i in 0..prune_batch_size {
            assert!(state_store
                .get_state_value_with_proof_by_version(&key, i as u64)
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
fn test_state_store_pruner_partial_version() {
    // ```text
    // | batch    | 0      | 1             | 2             |
    // | address1 | value1 |               |               |
    // | address2 | value2 | value2_update |               |
    // | address3 |        | value3        | value3_update |
    // ```
    // The stale node indexes will have 4 entries in total.
    // ```
    // index: StaleNodeIndex { stale_since_version: 1, node_key: NodeKey { version: 0, nibble_path:  } }
    // index: StaleNodeIndex { stale_since_version: 1, node_key: NodeKey { version: 0, nibble_path: 2 } }
    // index: StaleNodeIndex { stale_since_version: 2, node_key: NodeKey { version: 1, nibble_path:  } }
    // index: StaleNodeIndex { stale_since_version: 2, node_key: NodeKey { version: 1, nibble_path: d } }
    // ```
    // On version 1, there are two entries, one changes address2 and the other changes the root node.
    // On version 2, there are two entries, one changes address3 and the other changes the root node.
    let key1 = StateKey::Raw(String::from("test_key1").into_bytes());
    let key2 = StateKey::Raw(String::from("test_key2").into_bytes());
    let key3 = StateKey::Raw(String::from("test_key3").into_bytes());

    let value1 = StateValue::from(String::from("test_val1").into_bytes());
    let value2 = StateValue::from(String::from("test_val2").into_bytes());
    let value2_update = StateValue::from(String::from("test_val2_update").into_bytes());
    let value3 = StateValue::from(String::from("test_val3").into_bytes());
    let value3_update = StateValue::from(String::from("test_val3_update").into_bytes());

    let prune_batch_size = 1;
    let tmp_dir = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp_dir);
    let state_store = &aptos_db.state_store;
    let pruner = Pruner::new(
        Arc::clone(&aptos_db.ledger_db),
        Arc::clone(&aptos_db.state_merkle_db),
        StoragePrunerConfig {
            state_store_prune_window: Some(0),
            ledger_prune_window: Some(0),
            ledger_pruning_batch_size: prune_batch_size,
            state_store_pruning_batch_size: prune_batch_size,
        },
    );

    let _root0 = put_value_set(
        &aptos_db.ledger_db,
        state_store,
        vec![(key1.clone(), value1.clone()), (key2.clone(), value2)],
        0, /* version */
    );
    let _root1 = put_value_set(
        &aptos_db.ledger_db,
        state_store,
        vec![
            (key2.clone(), value2_update.clone()),
            (key3.clone(), value3.clone()),
        ],
        1, /* version */
    );
    let _root2 = put_value_set(
        &aptos_db.ledger_db,
        state_store,
        vec![(key3.clone(), value3_update.clone())],
        2, /* version */
    );

    // Prune till version=0. This should basically be a no-op
    {
        pruner
            .wake_and_wait_state_pruner(0 /* latest_version */)
            .unwrap();
        verify_state_in_store(state_store, key1.clone(), Some(&value1), 1);
        verify_state_in_store(state_store, key2.clone(), Some(&value2_update), 1);
        verify_state_in_store(state_store, key3.clone(), Some(&value3), 1);
    }

    // Test for batched pruning, since we use a batch size of 1, updating the latest version to 1
    // should prune 1 stale node with the version 0.
    {
        assert!(pruner
            .wake_and_wait_state_pruner(1 /* latest_version */,)
            .is_ok());
        assert!(state_store
            .get_state_value_with_proof_by_version(&key1, 0_u64)
            .is_err());
        // root1 is still there.
        verify_state_in_store(state_store, key1.clone(), Some(&value1), 1);
        verify_state_in_store(state_store, key2.clone(), Some(&value2_update), 1);
        verify_state_in_store(state_store, key3.clone(), Some(&value3), 1);
    }
    // Prune 3 more times. All version 0 and 1 stale nodes should be gone.
    {
        assert!(pruner
            .wake_and_wait_state_pruner(2 /* latest_version */,)
            .is_ok());
        assert!(pruner
            .wake_and_wait_state_pruner(2 /* latest_version */,)
            .is_ok());

        assert!(pruner
            .wake_and_wait_state_pruner(2 /* latest_version */,)
            .is_ok());
        assert!(state_store
            .get_state_value_with_proof_by_version(&key1, 0_u64)
            .is_err());
        assert!(state_store
            .get_state_value_with_proof_by_version(&key2, 1_u64)
            .is_err());
        // root2 is still there.
        verify_state_in_store(state_store, key1, Some(&value1), 2);
        verify_state_in_store(state_store, key2, Some(&value2_update), 2);
        verify_state_in_store(state_store, key3, Some(&value3_update), 2);
    }

    // Make sure all stale indices are gone.
    assert_eq!(
        aptos_db
            .state_merkle_db
            .iter::<StaleNodeIndexSchema>(ReadOptions::default())
            .unwrap()
            .collect::<Vec<_>>()
            .len(),
        0
    );
}

#[test]
fn test_state_store_pruner_disabled() {
    let key = StateKey::Raw(String::from("test_key1").into_bytes());

    let prune_batch_size = 10;
    let num_versions = 25;
    let tmp_dir = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp_dir);
    let state_store = &aptos_db.state_store;
    let pruner = Pruner::new(
        Arc::clone(&aptos_db.ledger_db),
        Arc::clone(&aptos_db.state_merkle_db),
        StoragePrunerConfig {
            state_store_prune_window: None,
            ledger_prune_window: Some(0),
            ledger_pruning_batch_size: prune_batch_size,
            state_store_pruning_batch_size: prune_batch_size,
        },
    );

    let mut root_hashes = vec![];
    // Insert 25 values in the db.
    for i in 0..num_versions {
        let value = StateValue::from(vec![i as u8]);
        root_hashes.push(put_value_set(
            &aptos_db.ledger_db,
            state_store,
            vec![(key.clone(), value.clone())],
            i as u64, /* version */
        ));
    }

    // Prune till version=0. This should basically be a no-op
    {
        pruner
            .ensure_disabled(PrunerIndex::StateStorePrunerIndex)
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

    // Notify the pruner to update the version to be 10 - since we use a batch size of 10,
    // we expect versions 0 to 9 to be pruned.
    {
        pruner
            .ensure_disabled(PrunerIndex::StateStorePrunerIndex)
            .unwrap();
        for i in 0..prune_batch_size {
            assert!(state_store
                .get_state_value_with_proof_by_version(&key, i as u64)
                .is_ok());
        }
        for i in 0..num_versions as usize {
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
    let db = aptos_db.ledger_db;
    let state_store = &aptos_db.state_store;

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
        let worker = StatePrunerWorker::new(
            Arc::clone(&aptos_db.state_merkle_db),
            command_receiver,
            Arc::new(Mutex::new(Some(0))), /* progress */
            StoragePrunerConfig {
                state_store_prune_window: Some(1),
                ledger_prune_window: Some(1),
                ledger_pruning_batch_size: 100,
                state_store_pruning_batch_size: 100,
            },
        );
        command_sender
            .send(db_pruner::Command::Prune {
                target_db_version: Some(1),
            })
            .unwrap();
        command_sender
            .send(db_pruner::Command::Prune {
                target_db_version: Some(2),
            })
            .unwrap();
        command_sender.send(db_pruner::Command::Quit).unwrap();
        // Worker quits immediately although `Command::Quit` is not the first command sent.
        worker.work();
        verify_state_in_store(state_store, key.clone(), Some(&value0), 0);
        verify_state_in_store(state_store, key.clone(), Some(&value1), 1);
        verify_state_in_store(state_store, key, Some(&value2), 2);
    }
}
