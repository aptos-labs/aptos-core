// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod speculative_state_workflow;

use super::*;
use crate::{
    db::test_helper::{arb_state_kv_sets_with_genesis, update_store},
    schema::jellyfish_merkle_node::JellyfishMerkleNodeSchema,
    state_restore::StateSnapshotRestore,
    AptosDB,
};
use aptos_jellyfish_merkle::{
    node_type::{Node, NodeKey},
    TreeReader,
};
use aptos_storage_interface::{DbReader, DbWriter, StateSnapshotReceiver};
use aptos_temppath::TempPath;
use aptos_types::nibble::nibble_path::NibblePath;
use proptest::{collection::hash_map, prelude::*};
use std::collections::BTreeMap;

fn put_value_set(
    state_store: &StateStore,
    value_set: Vec<(StateKey, StateValue)>,
    version: Version,
) -> HashValue {
    state_store.commit_block_for_test(version, [value_set.into_iter().map(|(k, v)| (k, Some(v)))])
}

fn verify_value_and_proof(
    store: &StateStore,
    key: StateKey,
    expected_value: Option<&StateValue>,
    version: Version,
    root: HashValue,
) {
    verify_value_and_proof_in_store(store, key.clone(), expected_value, version, root);
    verify_value_index_in_store(store, key, expected_value, version);
}

fn verify_value_and_proof_in_store(
    store: &StateStore,
    key: StateKey,
    expected_value: Option<&StateValue>,
    version: Version,
    root: HashValue,
) {
    let (value, proof) = store
        .get_state_value_with_proof_by_version(&key, version)
        .unwrap();
    assert_eq!(value.as_ref(), expected_value);
    proof.verify(root, key.hash(), value.as_ref()).unwrap();
}

fn verify_value_index_in_store(
    store: &StateStore,
    key: StateKey,
    expected_value: Option<&StateValue>,
    version: Version,
) {
    let value = store.get_state_value_by_version(&key, version).unwrap();
    assert_eq!(value.as_ref(), expected_value);
}

#[test]
fn test_empty_store() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let store = &db.state_store;
    let key = StateKey::raw(b"test_key");
    assert!(store
        .get_state_value_with_proof_by_version(&key, 0)
        .is_err());
}

#[test]
fn test_state_store_reader_writer() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let store = &db.state_store;
    let key1 = StateKey::raw(b"test_key1");
    let key2 = StateKey::raw(b"test_key2");
    let key3 = StateKey::raw(b"test_key3");

    let value1 = StateValue::from(String::from("test_val1").into_bytes());
    let value1_update = StateValue::from(String::from("test_val1_update").into_bytes());
    let value2 = StateValue::from(String::from("test_val2").into_bytes());
    let value3 = StateValue::from(String::from("test_val3").into_bytes());

    // Insert address1 with value 1 and verify new states.
    let mut root = put_value_set(
        store,
        vec![(key1.clone(), value1.clone())],
        0, /* version */
    );
    verify_value_and_proof(store, key1.clone(), Some(&value1), 0, root);

    verify_value_and_proof(store, key2.clone(), None, 0, root);
    verify_value_and_proof(store, key3.clone(), None, 0, root);

    // Insert address 1 with updated value1, address2 with value 2 and address3 with value3 and
    // verify new states.

    root = put_value_set(
        store,
        vec![
            (key1.clone(), value1_update.clone()),
            (key2.clone(), value2.clone()),
            (key3.clone(), value3.clone()),
        ],
        1, /* version */
    );

    verify_value_and_proof(store, key1, Some(&value1_update), 1, root);
    verify_value_and_proof(store, key2, Some(&value2), 1, root);
    verify_value_and_proof(store, key3, Some(&value3), 1, root);
}

#[test]
pub fn test_get_state_snapshot_before() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let store = &db.state_store;

    // Empty store
    assert_eq!(store.get_state_snapshot_before(0).unwrap(), None,);

    // put in genesis (version 0)
    let kv = vec![(
        StateKey::raw(b"key"),
        Some(StateValue::from(b"value".to_vec())),
    )];
    let hash = store.commit_block_for_test(0, [kv.clone()]);
    assert_eq!(store.get_state_snapshot_before(0).unwrap(), None);
    assert_eq!(store.get_state_snapshot_before(1).unwrap(), Some((0, hash)));
    assert_eq!(store.get_state_snapshot_before(2).unwrap(), Some((0, hash)));

    // put in snapshot at version 2
    let hash = store.commit_block_for_test(1, [vec![], kv]);
    assert_eq!(store.get_state_snapshot_before(4).unwrap(), Some((2, hash)));
    assert_eq!(store.get_state_snapshot_before(3).unwrap(), Some((2, hash)));
    assert_eq!(store.get_state_snapshot_before(2).unwrap(), Some((0, hash)));
    assert_eq!(store.get_state_snapshot_before(1).unwrap(), Some((0, hash)));
    assert_eq!(store.get_state_snapshot_before(0).unwrap(), None);

    // Only consider a version as available when the root node is there.
    // Here we are adding another non-root node, and removing the root node, to verify if there is
    // a node at version X but the root node at version X doesn't exist, we shouldn't return
    // version X.
    let mut batch = SchemaBatch::new();
    batch
        .put::<JellyfishMerkleNodeSchema>(
            &NodeKey::new(2, NibblePath::new_odd(vec![0])),
            &Node::Null,
        )
        .unwrap();
    db.state_merkle_db()
        .metadata_db()
        .write_schemas(batch)
        .unwrap();

    assert_eq!(
        db.state_merkle_db()
            .get_state_snapshot_version_before(4)
            .unwrap(),
        Some(2)
    );

    let mut batch = SchemaBatch::new();
    batch
        .delete::<JellyfishMerkleNodeSchema>(&NodeKey::new_empty_path(2))
        .unwrap();
    db.state_merkle_db()
        .metadata_db()
        .write_schemas(batch)
        .unwrap();

    assert_eq!(
        db.state_merkle_db()
            .get_state_snapshot_version_before(4)
            .unwrap(),
        Some(0)
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_get_account_iter(
        kvs_per_version in arb_state_kv_sets_with_genesis(5, 3, 5)
    ) {
        let tmp_dir = TempPath::new();
        let db = AptosDB::new_for_test(&tmp_dir);
        let store = &db.state_store;
        for (ver, kvs) in kvs_per_version.iter().cloned().enumerate() {
            store.commit_block_for_test(ver as Version, [kvs]);
        }

        // Ordered by key hash, tracking the latest state (None if deleted)
        let mut expected: BTreeMap<HashValue, Option<(StateKey, StateValue)>> = BTreeMap::new();
        // Test iterator at each version.
        for (ver, kvs) in kvs_per_version.into_iter().enumerate() {
            expected.extend(kvs.into_iter().map(|(k, v)| (k.hash(), v.map(|v| (k, v)))));
            let actual = db
                .get_backup_handler()
                .get_state_item_iter(ver as Version, 0, usize::MAX)
                .unwrap()
                .collect::<Result<Vec<_>>>()
                .unwrap();

            prop_assert!(
                itertools::equal(
                    actual.iter(),
                    expected.iter().filter_map(|(_h, kv)| kv.as_ref())
                )
            );
        }
    }

    #[test]
    fn test_raw_restore(
        (input, batch1_size) in hash_map(any::<StateKey>(), any::<StateValue>(), 2..1000)
            .prop_flat_map(|input| {
                let len = input.len();
                (Just(input), 1..len)
            })
    ) {
        let tmp_dir1 = TempPath::new();
        let db1 = AptosDB::new_for_test(&tmp_dir1);
        let store1 = &db1.state_store;
        init_store(store1, input.clone().into_iter());

        let version = (input.len() - 1) as Version;
        let expected_root_hash = store1.get_root_hash(version).unwrap();

        let tmp_dir2 = TempPath::new();
        let db2 = AptosDB::new_for_test(&tmp_dir2);
        let store2 = &db2.state_store;

        let mut restore =
            StateSnapshotRestore::new(&store2.state_merkle_db, store2, version, expected_root_hash, true /* async_commit */, StateSnapshotRestoreMode::Default).unwrap();

        let mut ordered_input: Vec<_> = input
            .into_iter()
            .collect();
        ordered_input.sort_unstable_by_key(|(key, _value)| key.hash());

        let batch1: Vec<_> = ordered_input
            .clone()
            .into_iter()
            .take(batch1_size)
            .collect();
        let rightmost_of_batch1 = batch1.last().map(|(key, _value)| key.hash()).unwrap();
        let proof_of_batch1 = store1
            .get_value_range_proof(rightmost_of_batch1, version)
            .unwrap();

        restore.add_chunk(batch1, proof_of_batch1).unwrap();

        let batch2: Vec<_> = ordered_input
            .into_iter()
            .skip(batch1_size)
            .collect();
        let rightmost_of_batch2 = batch2.last().map(|(key, _value)| key.hash()).unwrap();
        let proof_of_batch2 = store1
            .get_value_range_proof(rightmost_of_batch2, version)
            .unwrap();

        restore.add_chunk(batch2, proof_of_batch2).unwrap();

        restore.finish().unwrap();

        let actual_root_hash = store2.get_root_hash(version).unwrap();
        prop_assert_eq!(actual_root_hash, expected_root_hash);
    }

    #[test]
    fn test_restore(
        (input, batch_size) in hash_map(any::<StateKey>(), any::<StateValue>(), 2..1000)
            .prop_flat_map(|input| {
                let len = input.len();
                (Just(input), 1..len*2)
            })
    ) {
        let tmp_dir1 = TempPath::new();
        let db1 = AptosDB::new_for_test(&tmp_dir1);
        let store1 = &db1.state_store;
        init_store(store1, input.clone().into_iter());

        let version = (input.len() - 1) as Version;
        let expected_root_hash = store1.get_root_hash(version).unwrap();
        prop_assert_eq!(
            store1.get_value_count(version).unwrap(),
            input.len()
        );

        let tmp_dir2 = TempPath::new();
        let db2 = AptosDB::new_for_test(&tmp_dir2);
        let store2 = &db2.state_store;

        let mut restore = store2.get_snapshot_receiver(version, expected_root_hash).unwrap();
        let mut current_idx = 0;
        while current_idx < input.len() {
            let chunk = store1.get_value_chunk_with_proof(version, current_idx, batch_size).unwrap();
            restore.add_chunk(chunk.raw_values, chunk.proof).unwrap();
            current_idx += batch_size;
        }

        restore.finish_box().unwrap();
        let actual_root_hash = store2.get_root_hash(version).unwrap();
        prop_assert_eq!(actual_root_hash, expected_root_hash);
        prop_assert_eq!(
            store2.get_value_count(version).unwrap(),
            input.len()
        );
    }

    #[test]
    fn test_get_rightmost_leaf_with_sharding(
        (input, batch1_size) in hash_map(any::<StateKey>(), any::<StateValue>(), 2..1000)
        .prop_flat_map(|input| {
            let len = input.len();
            (Just(input), 2..=len)
        })
    ) {
        let tmp_dir1 = TempPath::new();
        let db1 = AptosDB::new_for_test_with_sharding(&tmp_dir1, 1000);
        let store1 = &db1.state_store;
        init_store(store1, input.clone().into_iter());

        let version = (input.len() - 1) as Version;
        let expected_root_hash = store1.get_root_hash(version).unwrap();

        let tmp_dir2 = TempPath::new();
        let db2 = AptosDB::new_for_test_with_sharding(&tmp_dir2, 1000);


        let store2 = &db2.state_store;
        let mut restore =
            StateSnapshotRestore::new(&store2.state_merkle_db, store2, version, expected_root_hash, true, /* async_commit */ StateSnapshotRestoreMode::Default).unwrap();
        let max_hash = HashValue::new([0xff; HashValue::LENGTH]);
        let dummy_state_key = StateKey::raw(&[]);
        let (top_levels_batch, sharded_batches, _) = store2.state_merkle_db.merklize_value_set(vec![(max_hash, Some(&(HashValue::random(), dummy_state_key)))], 0, None, None).unwrap();
        store2.state_merkle_db.commit(version, top_levels_batch, sharded_batches).unwrap();
        assert!(store2.state_merkle_db.get_rightmost_leaf(version).unwrap().is_none());
        let mut ordered_input: Vec<_> = input
            .into_iter()
            .collect();
        ordered_input.sort_unstable_by_key(|(key, _value)| key.hash());

        let batch1: Vec<_> = ordered_input
            .into_iter()
            .take(batch1_size)
            .collect();
        let rightmost_of_batch1 = batch1.last().map(|(key, _value)| key.hash()).unwrap();
        let proof_of_batch1 = store1
            .get_value_range_proof(rightmost_of_batch1, version)
            .unwrap();

        restore.add_chunk(batch1, proof_of_batch1).unwrap();
        restore.wait_for_async_commit().unwrap();

        let expected = store2.state_merkle_db.get_rightmost_leaf_naive(version).unwrap();
        // When re-initializing the store, the rightmost leaf should exist indicating the progress
        let actual = store2.state_merkle_db.get_rightmost_leaf(version).unwrap();
        // ensure the rightmost leaf is not None
        prop_assert!(actual.is_some());
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn test_get_rightmost_leaf(
        (input, batch1_size) in hash_map(any::<StateKey>(), any::<StateValue>(), 2..1000)
            .prop_flat_map(|input| {
                let len = input.len();
                (Just(input), 1..len)
            })
    ) {
        let tmp_dir1 = TempPath::new();
        let db1 = AptosDB::new_for_test(&tmp_dir1);
        let store1 = &db1.state_store;
        init_store(store1, input.clone().into_iter());

        let version = (input.len() - 1) as Version;
        let expected_root_hash = store1.get_root_hash(version).unwrap();

        let tmp_dir2 = TempPath::new();
        let db2 = AptosDB::new_for_test(&tmp_dir2);
        let store2 = &db2.state_store;
        let mut restore =
            StateSnapshotRestore::new(&store2.state_merkle_db, store2, version, expected_root_hash, true, /* async_commit */ StateSnapshotRestoreMode::Default).unwrap();
        let max_hash = HashValue::new([0xff; HashValue::LENGTH]);
        let dummy_state_key = StateKey::raw(&[]);
        let (top_levels_batch, sharded_batches, _) = store2.state_merkle_db.merklize_value_set(vec![(max_hash, Some(&(HashValue::random(), dummy_state_key)))], 0, None, None).unwrap();
        store2.state_merkle_db.commit(version, top_levels_batch, sharded_batches).unwrap();
        assert!(store2.state_merkle_db.get_rightmost_leaf(version).unwrap().is_none());
        let mut ordered_input: Vec<_> = input
            .into_iter()
            .collect();
        ordered_input.sort_unstable_by_key(|(key, _value)| key.hash());

        let batch1: Vec<_> = ordered_input
            .into_iter()
            .take(batch1_size)
            .collect();
        let rightmost_of_batch1 = batch1.last().map(|(key, _value)| key.hash()).unwrap();
        let proof_of_batch1 = store1
            .get_value_range_proof(rightmost_of_batch1, version)
            .unwrap();

        restore.add_chunk(batch1, proof_of_batch1).unwrap();
        restore.wait_for_async_commit().unwrap();

        let expected = store2.state_merkle_db.get_rightmost_leaf_naive(version).unwrap();
        let actual = store2.state_merkle_db.get_rightmost_leaf(version).unwrap();

        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn test_get_usage(
        input in arb_state_kv_sets_with_genesis(5, 3, 5)
    ) {
        let tmp_dir = TempPath::new();
        let db = AptosDB::new_for_test(&tmp_dir);
        let store = &db.state_store;

        let mut version = 0;
        for batch in input {
            let next_version = version + batch.len() as Version;
            let root_hash = update_store(store, batch.into_iter(), version);

            let last_version = next_version - 1;
            let snapshot = db
                .get_backup_handler()
                .get_state_item_iter(last_version, 0, usize::MAX)
                .unwrap()
                .collect::<Result<Vec<_>>>()
                .unwrap();
            let (items, bytes) = snapshot.iter().fold((0, 0), |(items, bytes), (k, v)| {
                (items + 1, bytes + k.size() + v.size())
            });
            let expected_usage = StateStorageUsage::new(items, bytes);
            prop_assert_eq!(
                expected_usage,
                store.get_usage(Some(last_version)).unwrap(),
                "version: {} next_version: {}",
                version,
                next_version,
            );

            // Check db restore calculates usage correctly as well.
            let tmp_dir = TempPath::new();
            let db2 = AptosDB::new_for_test(&tmp_dir);
            let mut restore = db2.get_state_snapshot_receiver(100, root_hash).unwrap();
            let proof = if let Some((k, _v)) = snapshot.last() {
                db.get_backup_handler().get_account_state_range_proof(k.hash(), last_version).unwrap()
            } else {
                SparseMerkleRangeProof::new(vec![])
            };
            restore.add_chunk(snapshot, proof).unwrap();
            restore.finish_box().unwrap();
            prop_assert_eq!(
                expected_usage,
                db2.state_store.get_usage(Some(100)).unwrap(),
                "version: {} next_version: {}",
                version,
                next_version,
            );

            version = next_version;
        }

    }
}

// Initializes the state store by inserting one key at each version.
fn init_store(store: &StateStore, input: impl Iterator<Item = (StateKey, StateValue)>) {
    update_store(store, input.into_iter().map(|(k, v)| (k, Some(v))), 0);
}

/// Test that hot state KV data persisted via `commit_block_for_test` can be loaded on DB reopen.
#[test]
fn test_hot_state_kv_persist_and_load() {
    use aptos_config::config::{
        HotStateConfig, RocksdbConfigs, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
    };

    let tmp_dir = TempPath::new();
    let hot_config = HotStateConfig {
        max_items_per_shard: 100,
        refresh_interval_versions: 100,
        delete_on_restart: false,
        compute_root_hash: true,
    };

    let key1 = StateKey::raw(b"test_key1");
    let key2 = StateKey::raw(b"test_key2");
    let key3 = StateKey::raw(b"test_key3");
    let val1 = StateValue::from(b"test_val1".to_vec());
    let val2 = StateValue::from(b"test_val2".to_vec());
    let val3 = StateValue::from(b"test_val3".to_vec());

    let hot_root_hash;
    let cold_root_hash;

    // Phase 1: Write blocks and persist hot state KV data.
    {
        let db = AptosDB::open(
            StorageDirPaths::from_path(&tmp_dir),
            false,
            NO_OP_STORAGE_PRUNER_CONFIG,
            RocksdbConfigs::default(),
            BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            None,
            hot_config,
        )
        .expect("Failed to open AptosDB");

        let store = &db.state_store;

        // Version 0: write key1, key2
        store.commit_block_for_test(0, [vec![
            (key1.clone(), Some(val1.clone())),
            (key2.clone(), Some(val2.clone())),
        ]]);

        // Version 1: write key3, update key1
        let val1_updated = StateValue::from(b"test_val1_updated".to_vec());
        store.commit_block_for_test(1, [vec![
            (key3.clone(), Some(val3.clone())),
            (key1.clone(), Some(val1_updated.clone())),
        ]]);

        // Record hashes for verification after reload.
        let current = store.current_state_locked().clone();
        hot_root_hash = current.last_checkpoint().summary().hot_root_hash();
        cold_root_hash = current.last_checkpoint().summary().root_hash();

        // Ensure the hot root hash is non-placeholder (hot state was actually computed).
        assert_ne!(
            hot_root_hash, *SPARSE_MERKLE_PLACEHOLDER_HASH,
            "Hot root hash should be non-placeholder after writing with hot state enabled"
        );
    }
    // DB is dropped/closed here.

    // Phase 2: Reopen DB and verify hot state was loaded from persisted KV data.
    {
        let db = AptosDB::open(
            StorageDirPaths::from_path(&tmp_dir),
            false,
            NO_OP_STORAGE_PRUNER_CONFIG,
            RocksdbConfigs::default(),
            BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            None,
            hot_config,
        )
        .expect("Failed to reopen AptosDB");

        let store = &db.state_store;

        let current = store.current_state_locked().clone();
        assert_eq!(
            current.last_checkpoint().summary().root_hash(),
            cold_root_hash,
            "Cold root hash mismatch after reload"
        );

        // Verify hot root hash matches — this confirms hot state was loaded, not started empty.
        assert_eq!(
            current.last_checkpoint().summary().hot_root_hash(),
            hot_root_hash,
            "Hot root hash mismatch after reload — hot state was not loaded correctly"
        );

        // Verify that hot state DashMaps are populated by querying through the HotStateView.
        let hot_state = store.persisted_state.get_hot_state();
        let (view, committed_state) = hot_state.get_committed();

        // Committed state should be at version 2 (next_version after version 1 checkpoint).
        assert_eq!(committed_state.next_version(), 2);

        // Check that all 3 keys are in the hot state view.
        let slot1 = view
            .get_state_slot(&key1)
            .expect("key1 should be in hot state");
        assert!(slot1.is_hot());
        assert_eq!(
            slot1.as_state_value_opt(),
            Some(&StateValue::from(b"test_val1_updated".to_vec())),
            "key1 should have updated value"
        );

        let slot2 = view
            .get_state_slot(&key2)
            .expect("key2 should be in hot state");
        assert!(slot2.is_hot());
        assert_eq!(slot2.as_state_value_opt(), Some(&val2));

        let slot3 = view
            .get_state_slot(&key3)
            .expect("key3 should be in hot state");
        assert!(slot3.is_hot());
        assert_eq!(slot3.as_state_value_opt(), Some(&val3));

        // Verify DashMaps directly — total entry count across all shards.
        let total_entries: usize = (0..NUM_STATE_SHARDS)
            .map(|shard_id| hot_state.get_all_entries(shard_id).len())
            .sum();
        assert_eq!(
            total_entries, 3,
            "Expected 3 entries in DashMaps after loading"
        );
    }
}
