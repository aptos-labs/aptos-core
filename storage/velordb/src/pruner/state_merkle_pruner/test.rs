// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db::{
        test_helper::{arb_state_kv_sets_with_genesis, update_store},
        VelorDB,
    },
    pruner::{PrunerManager, StateKvPrunerManager, StateMerklePrunerManager},
    schema::{
        stale_node_index::StaleNodeIndexSchema,
        stale_state_value_index::StaleStateValueIndexSchema,
        stale_state_value_index_by_key_hash::StaleStateValueIndexByKeyHashSchema,
    },
    state_merkle_db::StateMerkleDb,
    state_store::StateStore,
};
use velor_config::config::{LedgerPrunerConfig, StateMerklePrunerConfig};
use velor_crypto::{hash::CryptoHash, HashValue};
use velor_storage_interface::DbReader;
use velor_temppath::TempPath;
use velor_types::{
    state_store::{
        state_key::StateKey,
        state_value::{StaleStateValueByKeyHashIndex, StaleStateValueIndex, StateValue},
        NUM_STATE_SHARDS,
    },
    transaction::Version,
};
use proptest::{prelude::*, proptest};
use std::{collections::HashMap, sync::Arc};

fn put_value_set(
    state_store: &StateStore,
    value_set: Vec<(StateKey, StateValue)>,
    version: Version,
) -> HashValue {
    state_store.commit_block_for_test(version, [value_set.into_iter().map(|(k, v)| (k, Some(v)))])
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

fn create_state_merkle_pruner_manager(
    state_merkle_db: &Arc<StateMerkleDb>,
    prune_batch_size: usize,
) -> StateMerklePrunerManager<StaleNodeIndexSchema> {
    StateMerklePrunerManager::new(Arc::clone(state_merkle_db), StateMerklePrunerConfig {
        enable: true,
        prune_window: 0,
        batch_size: prune_batch_size,
    })
}

#[test]
fn test_state_store_pruner() {
    let key = StateKey::raw(b"test_key1");

    let prune_batch_size = 10;
    let num_versions = 25;
    let tmp_dir = TempPath::new();
    let velor_db = VelorDB::new_for_test_no_cache(&tmp_dir);
    let state_store = &velor_db.state_store;

    let mut root_hashes = vec![];
    // Insert 25 values in the db.
    for i in 0..num_versions {
        let value = StateValue::from(vec![i as u8]);
        root_hashes.push(put_value_set(
            state_store,
            vec![(key.clone(), value.clone())],
            i, /* version */
        ));
    }

    // Prune till version=0. This should basically be a no-op. Create a new pruner everytime to
    // test the min_readable_version initialization logic.
    {
        let pruner =
            create_state_merkle_pruner_manager(&velor_db.state_merkle_db(), prune_batch_size);
        pruner.wake_and_wait_pruner(0 /* latest_version */).unwrap();
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
    // we expect versions 0 to 9 to be pruned. Create a new pruner everytime to test the
    // min_readable_version initialization logic.
    {
        let pruner =
            create_state_merkle_pruner_manager(&velor_db.state_merkle_db(), prune_batch_size);
        pruner
            .wake_and_wait_pruner(prune_batch_size as u64 /* latest_version */)
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
    let key1 = StateKey::raw(b"test_key1");
    let key2 = StateKey::raw(b"test_key2");
    let key3 = StateKey::raw(b"test_key3");

    let value1 = StateValue::from(String::from("test_val1").into_bytes());
    let value2 = StateValue::from(String::from("test_val2").into_bytes());
    let value2_update = StateValue::from(String::from("test_val2_update").into_bytes());
    let value3 = StateValue::from(String::from("test_val3").into_bytes());
    let value3_update = StateValue::from(String::from("test_val3_update").into_bytes());

    let prune_batch_size = 1;
    let tmp_dir = TempPath::new();
    let velor_db = VelorDB::new_for_test_with_sharding(&tmp_dir, 0);
    let state_store = &velor_db.state_store;

    let _root0 = put_value_set(
        state_store,
        vec![(key1.clone(), value1.clone()), (key2.clone(), value2)],
        0, /* version */
    );
    let _root1 = put_value_set(
        state_store,
        vec![
            (key2.clone(), value2_update.clone()),
            (key3.clone(), value3.clone()),
        ],
        1, /* version */
    );
    let _root2 = put_value_set(
        state_store,
        vec![(key3.clone(), value3_update.clone())],
        2, /* version */
    );

    // Prune till version=0. This should basically be a no-op. Create a new pruner every time
    // to test the min_readable_version initialization logic.
    {
        let pruner =
            create_state_merkle_pruner_manager(&velor_db.state_merkle_db(), prune_batch_size);
        pruner.wake_and_wait_pruner(0 /* latest_version */).unwrap();
        verify_state_in_store(state_store, key1.clone(), Some(&value1), 1);
        verify_state_in_store(state_store, key2.clone(), Some(&value2_update), 1);
        verify_state_in_store(state_store, key3.clone(), Some(&value3), 1);
    }

    // Test for batched pruning, since we use a batch size of 1, updating the latest version to 1
    // should prune 1 stale node with the version 0. Create a new pruner everytime to test the
    // min_readable_version initialization logic.
    {
        let pruner =
            create_state_merkle_pruner_manager(&velor_db.state_merkle_db(), prune_batch_size);
        assert!(pruner.wake_and_wait_pruner(1 /* latest_version */,).is_ok());
        assert!(state_store
            .get_state_value_with_proof_by_version(&key1, 0_u64)
            .is_err());
        // root1 is still there.
        verify_state_in_store(state_store, key1.clone(), Some(&value1), 1);
        verify_state_in_store(state_store, key2.clone(), Some(&value2_update), 1);
        verify_state_in_store(state_store, key3.clone(), Some(&value3), 1);
    }
    // Prune 3 more times. All version 0 and 1 stale nodes should be gone. Create a new pruner
    // everytime to test the min_readable_version initialization logic.
    {
        let pruner =
            create_state_merkle_pruner_manager(&velor_db.state_merkle_db(), prune_batch_size);
        assert!(pruner.wake_and_wait_pruner(2 /* latest_version */,).is_ok());
        assert!(pruner.wake_and_wait_pruner(2 /* latest_version */,).is_ok());

        assert!(pruner.wake_and_wait_pruner(2 /* latest_version */,).is_ok());
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
    //
    assert_eq!(
        velor_db
            .state_merkle_db()
            .metadata_db()
            .iter::<StaleNodeIndexSchema>()
            .unwrap()
            .count(),
        0
    );

    if velor_db.state_merkle_db().sharding_enabled() {
        for i in 0..NUM_STATE_SHARDS {
            assert_eq!(
                velor_db
                    .state_merkle_db()
                    .db_shard(i)
                    .iter::<StaleNodeIndexSchema>()
                    .unwrap()
                    .count(),
                0
            );
        }
    }
}

#[test]
fn test_state_store_pruner_disabled() {
    let key = StateKey::raw(b"test_key1");

    let prune_batch_size = 10;
    let num_versions = 25;
    let tmp_dir = TempPath::new();
    let velor_db = VelorDB::new_for_test(&tmp_dir);
    let state_store = &velor_db.state_store;

    let mut root_hashes = vec![];
    // Insert 25 values in the db.
    for i in 0..num_versions {
        let value = StateValue::from(vec![i as u8]);
        root_hashes.push(put_value_set(
            state_store,
            vec![(key.clone(), value.clone())],
            i, /* version */
        ));
    }

    // Prune till version=0. This should basically be a no-op
    {
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

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_state_value_pruner(
        input in arb_state_kv_sets_with_genesis(5, 3, 5),
    ) {
        verify_state_value_pruner(input);
    }
}

fn verify_state_value_pruner(inputs: Vec<Vec<(StateKey, Option<StateValue>)>>) {
    let tmp_dir = TempPath::new();
    let db = VelorDB::new_for_test(&tmp_dir);
    let store = &db.state_store;

    let mut version = 0;
    let mut current_state_values = HashMap::new();
    let pruner = StateKvPrunerManager::new(Arc::clone(&db.state_kv_db), LedgerPrunerConfig {
        enable: true,
        prune_window: 0,
        batch_size: 1,
        user_pruning_window_offset: 0,
    });
    for batch in inputs {
        update_store(store, batch.clone().into_iter(), version);
        for (k, v) in batch.iter() {
            if let Some((old_version, old_v_opt)) =
                current_state_values.insert(k.clone(), (version, v.clone()))
            {
                pruner
                    .wake_and_wait_pruner(version /* latest_version */)
                    .unwrap();
                if version > 0 {
                    verify_state_value(
                        vec![(k, &(old_version, old_v_opt))].into_iter(),
                        version - 1,
                        store,
                        true,
                    );
                }
            }
            verify_state_value(current_state_values.iter(), version, store, false);
            version += 1;
        }
    }
}

fn verify_state_value<'a, I: Iterator<Item = (&'a StateKey, &'a (Version, Option<StateValue>))>>(
    kvs: I,
    version: Version,
    state_store: &Arc<StateStore>,
    pruned: bool,
) {
    for (k, (old_version, v)) in kvs {
        let v_from_db = state_store.get_state_value_by_version(k, version).unwrap();
        assert_eq!(&v_from_db, if pruned { &None } else { v });
        let enable_sharding = state_store.state_kv_db.enabled_sharding();
        if pruned {
            if !enable_sharding {
                assert!(state_store
                    .state_kv_db
                    .db_shard(k.get_shard_id())
                    .get::<StaleStateValueIndexSchema>(&StaleStateValueIndex {
                        stale_since_version: version,
                        version: *old_version,
                        state_key: k.clone()
                    })
                    .unwrap()
                    .is_none());
            } else {
                assert!(state_store
                    .state_kv_db
                    .db_shard(k.get_shard_id())
                    .get::<StaleStateValueIndexByKeyHashSchema>(&StaleStateValueByKeyHashIndex {
                        stale_since_version: version,
                        version: *old_version,
                        state_key_hash: k.hash()
                    })
                    .unwrap()
                    .is_none());
            }
        }
    }
}
