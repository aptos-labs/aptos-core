// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashSet, convert::TryFrom};

use proptest::{
    collection::{hash_map, vec},
    prelude::*,
};

use aptos_jellyfish_merkle::restore::JellyfishMerkleRestore;
use aptos_temppath::TempPath;
use aptos_types::{
    access_path::AccessPath, account_address::AccountAddress, account_state_blob::AccountStateBlob,
    state_store::state_key::StateKeyTag,
};
use storage_interface::StateSnapshotReceiver;

use crate::{pruner, AptosDB};

use super::*;

fn put_account_state_set(
    store: &StateStore,
    account_state_set: Vec<(AccountAddress, AccountStateBlob)>,
    version: Version,
    expected_new_nodes: usize,
    expected_stale_nodes: usize,
    expected_stale_leaves: usize,
) -> HashValue {
    let mut cs = ChangeSet::new();
    let expected_new_leaves = account_state_set.len();
    let value_set: HashMap<_, _> = account_state_set
        .iter()
        .map(|(address, blob)| {
            (
                StateKey::AccountAddressKey(*address),
                StateValue::from(blob.clone()),
            )
        })
        .collect();
    let root = store
        .put_value_sets(vec![&value_set], None, version, &mut cs)
        .unwrap()[0];
    let bumps = cs.counter_bumps(version);
    assert_eq!(bumps.get(LedgerCounter::NewStateNodes), expected_new_nodes);
    assert_eq!(
        bumps.get(LedgerCounter::StaleStateNodes),
        expected_stale_nodes
    );
    assert_eq!(
        bumps.get(LedgerCounter::NewStateLeaves),
        expected_new_leaves
    );
    assert_eq!(
        bumps.get(LedgerCounter::StaleStateLeaves),
        expected_stale_leaves
    );

    store.db.write_schemas(cs.batch).unwrap();
    root
}

fn put_value_set(
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
    state_store.db.write_schemas(cs.batch).unwrap();
    root
}

fn prune_stale_indices(
    store: &StateStore,
    least_readable_version: Version,
    target_least_readable_version: Version,
    limit: usize,
) {
    pruner::state_store::prune_state_store(
        Arc::clone(&store.db),
        least_readable_version,
        target_least_readable_version,
        limit,
    )
    .unwrap();
}

fn verify_value_and_proof(
    store: &StateStore,
    address: AccountAddress,
    expected_value: Option<&AccountStateBlob>,
    version: Version,
    root: HashValue,
) {
    verify_value_and_proof_in_store(store, address, expected_value, version, root);
    verify_value_index_in_store(store, address, expected_value, version);
}

fn verify_value_and_proof_in_store(
    store: &StateStore,
    address: AccountAddress,
    expected_value: Option<&AccountStateBlob>,
    version: Version,
    root: HashValue,
) {
    let (value, proof) = store
        .get_value_with_proof_by_version(&StateKey::AccountAddressKey(address), version)
        .unwrap();
    assert_eq!(
        value
            .clone()
            .map(|x| AccountStateBlob::try_from(x).unwrap())
            .as_ref(),
        expected_value
    );
    proof
        .verify(
            root,
            StateKey::AccountAddressKey(address).hash(),
            value.as_ref(),
        )
        .unwrap();
}

fn verify_value_index_in_store(
    store: &StateStore,
    address: AccountAddress,
    expected_value: Option<&AccountStateBlob>,
    version: Version,
) {
    let value = store
        .get_value_by_version(&StateKey::AccountAddressKey(address), version)
        .unwrap();
    assert_eq!(
        value
            .map(|x| AccountStateBlob::try_from(x).unwrap())
            .as_ref(),
        expected_value
    );
}

#[test]
fn test_empty_store() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let store = &db.state_store;
    let address = AccountAddress::new([1u8; AccountAddress::LENGTH]);
    assert!(store
        .get_value_with_proof_by_version(&StateKey::AccountAddressKey(address), 0)
        .is_err());
}

#[test]
fn test_state_store_reader_writer() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let store = &db.state_store;
    let address1 = AccountAddress::new([1u8; AccountAddress::LENGTH]);
    let address2 = AccountAddress::new([2u8; AccountAddress::LENGTH]);
    let address3 = AccountAddress::new([3u8; AccountAddress::LENGTH]);
    let value1 = AccountStateBlob::from(vec![0x01]);
    let value1_update = AccountStateBlob::from(vec![0x00]);
    let value2 = AccountStateBlob::from(vec![0x02]);
    let value3 = AccountStateBlob::from(vec![0x03]);

    // Insert address1 with value 1 and verify new states.
    let mut root = put_account_state_set(
        store,
        vec![(address1, value1.clone())],
        0, /* version */
        1, /* expected_nodes_created */
        0, /* expected_nodes_retired */
        0, /* expected_blobs_retired */
    );
    verify_value_and_proof(store, address1, Some(&value1), 0, root);

    verify_value_and_proof(store, address2, None, 0, root);
    verify_value_and_proof(store, address3, None, 0, root);

    // Insert address 1 with updated value1, address2 with value 2 and address3 with value3 and
    // verify new states.

    root = put_account_state_set(
        store,
        vec![
            (address1, value1_update.clone()),
            (address2, value2.clone()),
            (address3, value3.clone()),
        ],
        1, /* version */
        4, /* expected_nodes_created */
        1, /* expected_nodes_retired */
        1, /* expected_blobs_retired */
    );

    verify_value_and_proof(store, address1, Some(&value1_update), 1, root);
    verify_value_and_proof(store, address2, Some(&value2), 1, root);
    verify_value_and_proof(store, address3, Some(&value3), 1, root);
}

#[test]
fn test_get_values_by_key_prefix() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let store = &db.state_store;
    let address = AccountAddress::new([12u8; AccountAddress::LENGTH]);

    let key1 = StateKey::AccessPath(AccessPath::new(address, b"state_key1".to_vec()));
    let key2 = StateKey::AccessPath(AccessPath::new(address, b"state_key2".to_vec()));

    let value1_v0 = StateValue::from(String::from("value1_v0").into_bytes());
    let value2_v0 = StateValue::from(String::from("value2_v0").into_bytes());

    let account_key_prefx = StateKeyPrefix::new(StateKeyTag::AccessPath, address.to_vec());

    put_value_set(
        store,
        vec![
            (key1.clone(), value1_v0.clone()),
            (key2.clone(), value2_v0.clone()),
        ],
        0,
    );

    let key_value_map = store
        .get_values_by_key_prefix(&account_key_prefx, 0)
        .unwrap();
    assert_eq!(key_value_map.len(), 2);
    assert_eq!(*key_value_map.get(&key1).unwrap(), value1_v0);
    assert_eq!(*key_value_map.get(&key2).unwrap(), value2_v0);

    let key4 = StateKey::AccessPath(AccessPath::new(address, b"state_key4".to_vec()));

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
    let key_value_map = store
        .get_values_by_key_prefix(&account_key_prefx, 0)
        .unwrap();
    assert_eq!(key_value_map.len(), 2);
    assert_eq!(*key_value_map.get(&key1).unwrap(), value1_v0);
    assert_eq!(*key_value_map.get(&key2).unwrap(), value2_v0);

    // Ensure that key value map for version 1 returns value for key1 at version 0.
    let key_value_map = store
        .get_values_by_key_prefix(&account_key_prefx, 1)
        .unwrap();
    assert_eq!(key_value_map.len(), 3);
    assert_eq!(*key_value_map.get(&key1).unwrap(), value1_v0);
    assert_eq!(*key_value_map.get(&key2).unwrap(), value2_v1);
    assert_eq!(*key_value_map.get(&key4).unwrap(), value4_v1);

    // Add values for one more account and verify the state
    let address1 = AccountAddress::new([22u8; AccountAddress::LENGTH]);
    let key5 = StateKey::AccessPath(AccessPath::new(address1, b"state_key5".to_vec()));
    let value5_v2 = StateValue::from(String::from("value5_v2").into_bytes());

    let account1_key_prefx = StateKeyPrefix::new(StateKeyTag::AccessPath, address1.to_vec());

    put_value_set(store, vec![(key5.clone(), value5_v2.clone())], 2);

    // address1 did not exist in version 0 and 1.
    let key_value_map = store
        .get_values_by_key_prefix(&account1_key_prefx, 0)
        .unwrap();
    assert_eq!(key_value_map.len(), 0);

    let key_value_map = store
        .get_values_by_key_prefix(&account1_key_prefx, 1)
        .unwrap();
    assert_eq!(key_value_map.len(), 0);

    let key_value_map = store
        .get_values_by_key_prefix(&account1_key_prefx, 2)
        .unwrap();
    assert_eq!(key_value_map.len(), 1);
    assert_eq!(*key_value_map.get(&key5).unwrap(), value5_v2);
}

#[test]
fn test_retired_records() {
    let address1 = AccountAddress::new([1u8; AccountAddress::LENGTH]);
    let address2 = AccountAddress::new([2u8; AccountAddress::LENGTH]);
    let address3 = AccountAddress::new([3u8; AccountAddress::LENGTH]);
    let value1 = AccountStateBlob::from(vec![0x01]);
    let value2 = AccountStateBlob::from(vec![0x02]);
    let value2_update = AccountStateBlob::from(vec![0x12]);
    let value3 = AccountStateBlob::from(vec![0x03]);
    let value3_update = AccountStateBlob::from(vec![0x13]);

    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let store = &db.state_store;

    // Update.
    // ```text
    // | batch    | 0      | 1             | 2             |
    // | address1 | value1 |               |               |
    // | address2 | value2 | value2_update |               |
    // | address3 |        | value3        | value3_update |
    // ```
    let root0 = put_account_state_set(
        store,
        vec![(address1, value1.clone()), (address2, value2)],
        0, /* version */
        3, /* expected_nodes_created */
        0, /* expected_nodes_retired */
        0, /* expected_blobs_retired */
    );
    let root1 = put_account_state_set(
        store,
        vec![
            (address2, value2_update.clone()),
            (address3, value3.clone()),
        ],
        1, /* version */
        3, /* expected_nodes_created */
        2, /* expected_nodes_retired */
        1, /* expected_blobs_retired */
    );
    let root2 = put_account_state_set(
        store,
        vec![(address3, value3_update.clone())],
        2, /* version */
        2, /* expected_nodes_created */
        2, /* expected_nodes_retired */
        1, /* expected_blobs_retired */
    );

    // Verify.
    // Prune with limit=0, nothing is gone.
    {
        prune_stale_indices(
            store, 0, /* least_readable_version */
            1, /* target_least_readable_version */
            0, /* limit */
        );
        verify_value_and_proof(store, address1, Some(&value1), 0, root0);
    }
    // Prune till version=1.
    {
        prune_stale_indices(
            store, 0,   /* least_readable_version */
            1,   /* target_least_readable_version */
            100, /* limit */
        );
        // root0 is gone.
        assert!(store
            .get_value_with_proof_by_version(&StateKey::AccountAddressKey(address2), 0)
            .is_err());
        // root1 is still there.
        verify_value_and_proof(store, address1, Some(&value1), 1, root1);
        verify_value_and_proof(store, address2, Some(&value2_update), 1, root1);
        verify_value_and_proof(store, address3, Some(&value3), 1, root1);
    }
    // Prune till version=2.
    {
        prune_stale_indices(
            store, 1,   /* least_readable_version */
            2,   /* target_least_readable_version */
            100, /* limit */
        );
        // root1 is gone.
        assert!(store
            .get_value_with_proof_by_version(&StateKey::AccountAddressKey(address2), 1)
            .is_err());
        // root2 is still there.
        verify_value_and_proof(store, address1, Some(&value1), 2, root2);
        verify_value_and_proof(store, address2, Some(&value2_update), 2, root2);
        verify_value_and_proof(store, address3, Some(&value3_update), 2, root2);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_get_account_iter(
        input in hash_map(any::<StateKey>(), any::<StateValue>(), 1..200)
    ) {
        // Convert to a vector so iteration order becomes deterministic.
        let kvs: Vec<_> = input.into_iter().collect();

        let tmp_dir = TempPath::new();
        let db = AptosDB::new_for_test(&tmp_dir);
        let store = &db.state_store;
        init_store(store, kvs.clone().into_iter());

        // Test iterator at each version.
        for i in 0..kvs.len() {
            let actual_values = db
                .get_backup_handler()
                .get_account_iter(i as Version)
                .unwrap()
                .collect::<Result<Vec<_>>>()
                .unwrap();
            let mut expected_values: Vec<_> = kvs[..=i]
                .iter()
                .map(|(key, value)| (key.hash(), StateKeyAndValue::new(key.clone(), value.clone())))
                .collect();
            expected_values.sort_unstable_by_key(|item| item.0);
            prop_assert_eq!(actual_values, expected_values);
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
            JellyfishMerkleRestore::new(Arc::clone(store2), version, expected_root_hash).unwrap();

        let mut ordered_input: Vec<_> = input
            .into_iter()
            .map(|(addr, value)| (addr.hash(), value))
            .collect();
        ordered_input.sort_unstable_by_key(|(key, _value)| *key);

        let batch1: Vec<_> = ordered_input
            .clone()
            .into_iter()
            .take(batch1_size)
            .map(|(key, value)| (key, StateKeyAndValue::new(StateKey::Raw(vec![]), value)))
            .collect();
        let rightmost_of_batch1 = batch1.last().map(|(key, _value)| *key).unwrap();
        let proof_of_batch1 = store1
            .get_value_range_proof(rightmost_of_batch1, version)
            .unwrap();

        restore.add_chunk(batch1, proof_of_batch1).unwrap();

        let batch2: Vec<_> = ordered_input
            .into_iter()
            .skip(batch1_size)
            .map(|(key, value)| (key, StateKeyAndValue::new(StateKey::Raw(vec![]), value)))
            .collect();
        let rightmost_of_batch2 = batch2.last().map(|(key, _value)| *key).unwrap();
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
            JellyfishMerkleRestore::new(Arc::clone(store2), version, expected_root_hash).unwrap();

        let mut ordered_input: Vec<_> = input
            .into_iter()
            .map(|(addr, value)| (addr.hash(), value))
            .collect();
        ordered_input.sort_unstable_by_key(|(key, _value)| *key);

        let batch1: Vec<_> = ordered_input
            .into_iter()
            .take(batch1_size)
            .map(|(key, value)| (key, StateKeyAndValue::new(StateKey::Raw(vec![]), value)))
            .collect();
        let rightmost_of_batch1 = batch1.last().map(|(key, _value)| *key).unwrap();
        let proof_of_batch1 = store1
            .get_value_range_proof(rightmost_of_batch1, version)
            .unwrap();

        restore.add_chunk(batch1, proof_of_batch1).unwrap();

        let expected = store2.get_rightmost_leaf_naive().unwrap();
        let actual = store2.get_rightmost_leaf().unwrap();
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn test_get_account_count(
        input in vec((any::<StateKey>(), any::<StateValue>()), 1..200)
    ) {
        let version = (input.len() - 1) as Version;
        let account_count = input.iter().map(|(k, _)| k).collect::<HashSet<_>>().len();

        let tmp_dir = TempPath::new();
        let db = AptosDB::new_for_test(&tmp_dir);
        let store = &db.state_store;
        init_store(store, input.into_iter());
        assert_eq!(store.get_value_count(version).unwrap(), account_count);
    }
}

// Initializes the state store by inserting one key at each version.
fn init_store(store: &StateStore, input: impl Iterator<Item = (StateKey, StateValue)>) {
    update_store(store, input, 0);
}

fn update_store(
    store: &StateStore,
    input: impl Iterator<Item = (StateKey, StateValue)>,
    first_version: Version,
) {
    for (i, (key, value)) in input.enumerate() {
        let mut cs = ChangeSet::new();
        let value_state_set: HashMap<_, _> = std::iter::once((key, value)).collect();
        store
            .put_value_sets(
                vec![&value_state_set],
                None,
                first_version + i as Version,
                &mut cs,
            )
            .unwrap();
        store.db.write_schemas(cs.batch).unwrap();
    }
}
