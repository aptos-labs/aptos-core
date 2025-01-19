// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

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
use aptos_types::{
    account_address::AccountAddress,
    account_config::{AccountResource, ChainIdResource, CoinInfoResource, CoinStoreResource},
    nibble::nibble_path::NibblePath,
    state_store::state_key::inner::StateKeyTag,
    AptosCoinType,
};
use proptest::{collection::hash_map, prelude::*};
use std::collections::{BTreeMap, HashMap};

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

fn traverse_values(
    store: &StateStore,
    prefix: &StateKeyPrefix,
    version: Version,
) -> HashMap<StateKey, StateValue> {
    let mut ret = HashMap::new();
    let mut cursor = None;
    loop {
        let mut iter = store
            .get_prefixed_state_value_iterator(prefix, cursor.as_ref(), version)
            .unwrap();
        if let Some((k, v)) = iter.next().transpose().unwrap() {
            ret.insert(k, v);
        }
        cursor = iter.next().transpose().unwrap().map(|(k, _v)| k);
        if cursor.is_none() {
            return ret;
        }
    }
}

#[test]
fn test_get_values_by_key_prefix() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let store = &db.state_store;
    let address = AccountAddress::new([12u8; AccountAddress::LENGTH]);

    let key1 = StateKey::resource_typed::<AccountResource>(&address).unwrap();
    let key2 = StateKey::resource_typed::<ChainIdResource>(&address).unwrap();

    let value1_v0 = StateValue::from(String::from("value1_v0").into_bytes());
    let value2_v0 = StateValue::from(String::from("value2_v0").into_bytes());

    let account_key_prefix = StateKeyPrefix::new(StateKeyTag::AccessPath, address.to_vec());

    put_value_set(
        store,
        vec![
            (key1.clone(), value1_v0.clone()),
            (key2.clone(), value2_v0.clone()),
        ],
        0,
    );

    let key_value_map = traverse_values(store, &account_key_prefix, 0);
    assert_eq!(key_value_map.len(), 2);
    assert_eq!(*key_value_map.get(&key1).unwrap(), value1_v0);
    assert_eq!(*key_value_map.get(&key2).unwrap(), value2_v0);

    let key4 = StateKey::resource_typed::<CoinInfoResource<AptosCoinType>>(&address).unwrap();

    let value2_v1 = StateValue::from(String::from("value2_v1").into_bytes());
    let value4_v1 = StateValue::from(String::from("value4_v1").into_bytes());

    put_value_set(
        store,
        vec![
            (key2.clone(), value2_v1.clone()),
            (key4.clone(), value4_v1.clone()),
        ],
        1,
    );

    // Ensure that we still get only values for key1 and key2 for version 0 after the update
    let key_value_map = traverse_values(store, &account_key_prefix, 0);
    assert_eq!(key_value_map.len(), 2);
    assert_eq!(*key_value_map.get(&key1).unwrap(), value1_v0);
    assert_eq!(*key_value_map.get(&key2).unwrap(), value2_v0);

    // Ensure that key value map for version 1 returns value for key1 at version 0.
    let key_value_map = traverse_values(store, &account_key_prefix, 1);
    assert_eq!(key_value_map.len(), 3);
    assert_eq!(*key_value_map.get(&key1).unwrap(), value1_v0);
    assert_eq!(*key_value_map.get(&key2).unwrap(), value2_v1);
    assert_eq!(*key_value_map.get(&key4).unwrap(), value4_v1);

    // Add values for one more account and verify the state
    let address1 = AccountAddress::new([22u8; AccountAddress::LENGTH]);
    let key5 = StateKey::resource_typed::<CoinStoreResource<AptosCoinType>>(&address1).unwrap();
    let value5_v2 = StateValue::from(String::from("value5_v2").into_bytes());

    let account1_key_prefix = StateKeyPrefix::new(StateKeyTag::AccessPath, address1.to_vec());

    put_value_set(store, vec![(key5.clone(), value5_v2.clone())], 2);

    // address1 did not exist in version 0 and 1.
    let key_value_map = traverse_values(store, &account1_key_prefix, 0);
    assert_eq!(key_value_map.len(), 0);

    let key_value_map = traverse_values(store, &account1_key_prefix, 1);
    assert_eq!(key_value_map.len(), 0);

    let key_value_map = traverse_values(store, &account1_key_prefix, 2);
    assert_eq!(key_value_map.len(), 1);
    assert_eq!(*key_value_map.get(&key5).unwrap(), value5_v2);
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
