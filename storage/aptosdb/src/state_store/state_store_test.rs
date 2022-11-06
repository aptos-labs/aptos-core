// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    state_restore::StateSnapshotRestore,
    test_helper::{arb_state_kv_sets, update_store},
    AptosDB,
};
use aptos_jellyfish_merkle::TreeReader;
use aptos_temppath::TempPath;
use aptos_types::{
    access_path::AccessPath, account_address::AccountAddress, state_store::state_key::StateKeyTag,
};
use proptest::{collection::hash_map, prelude::*};
use storage_interface::{jmt_update_refs, jmt_updates, DbReader, DbWriter, StateSnapshotReceiver};

use super::*;

fn put_value_set(
    state_store: &StateStore,
    value_set: Vec<(StateKey, StateValue)>,
    version: Version,
    base_version: Option<Version>,
) -> HashValue {
    let value_set: HashMap<_, _> = value_set
        .iter()
        .map(|(key, value)| (key.clone(), Some(value.clone())))
        .collect();
    let jmt_updates = jmt_updates(&value_set);

    let root = state_store
        .merklize_value_set(jmt_update_refs(&jmt_updates), None, version, base_version)
        .unwrap();
    let mut batch = SchemaBatch::new();
    state_store
        .put_value_sets(
            vec![&value_set],
            version,
            StateStorageUsage::new_untracked(),
            &mut batch,
        )
        .unwrap();
    state_store.ledger_db.write_schemas(batch).unwrap();
    root
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
    let key = StateKey::Raw(String::from("test_key").into_bytes());
    assert!(store
        .get_state_value_with_proof_by_version(&key, 0)
        .is_err());
}

#[test]
fn test_state_store_reader_writer() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let store = &db.state_store;
    let key1 = StateKey::Raw(String::from("test_key1").into_bytes());
    let key2 = StateKey::Raw(String::from("test_key2").into_bytes());
    let key3 = StateKey::Raw(String::from("test_key3").into_bytes());

    let value1 = StateValue::from(String::from("test_val1").into_bytes());
    let value1_update = StateValue::from(String::from("test_val1_update").into_bytes());
    let value2 = StateValue::from(String::from("test_val2").into_bytes());
    let value3 = StateValue::from(String::from("test_val3").into_bytes());

    // Insert address1 with value 1 and verify new states.
    let mut root = put_value_set(
        store,
        vec![(key1.clone(), value1.clone())],
        0, /* version */
        None,
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
        Some(0),
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
        None,
    );

    let key_value_map = traverse_values(store, &account_key_prefx, 0);
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
        Some(0),
    );

    // Ensure that we still get only values for key1 and key2 for version 0 after the update
    let key_value_map = traverse_values(store, &account_key_prefx, 0);
    assert_eq!(key_value_map.len(), 2);
    assert_eq!(*key_value_map.get(&key1).unwrap(), value1_v0);
    assert_eq!(*key_value_map.get(&key2).unwrap(), value2_v0);

    // Ensure that key value map for version 1 returns value for key1 at version 0.
    let key_value_map = traverse_values(store, &account_key_prefx, 1);
    assert_eq!(key_value_map.len(), 3);
    assert_eq!(*key_value_map.get(&key1).unwrap(), value1_v0);
    assert_eq!(*key_value_map.get(&key2).unwrap(), value2_v1);
    assert_eq!(*key_value_map.get(&key4).unwrap(), value4_v1);

    // Add values for one more account and verify the state
    let address1 = AccountAddress::new([22u8; AccountAddress::LENGTH]);
    let key5 = StateKey::AccessPath(AccessPath::new(address1, b"state_key5".to_vec()));
    let value5_v2 = StateValue::from(String::from("value5_v2").into_bytes());

    let account1_key_prefx = StateKeyPrefix::new(StateKeyTag::AccessPath, address1.to_vec());

    put_value_set(store, vec![(key5.clone(), value5_v2.clone())], 2, Some(1));

    // address1 did not exist in version 0 and 1.
    let key_value_map = traverse_values(store, &account1_key_prefx, 0);
    assert_eq!(key_value_map.len(), 0);

    let key_value_map = traverse_values(store, &account1_key_prefx, 1);
    assert_eq!(key_value_map.len(), 0);

    let key_value_map = traverse_values(store, &account1_key_prefx, 2);
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

    // put in genesis
    let kv = vec![(
        StateKey::Raw(b"key".to_vec()),
        StateValue::from(b"value".to_vec()),
    )];
    let hash = put_value_set(store, kv.clone(), 0, None);
    assert_eq!(store.get_state_snapshot_before(0).unwrap(), None);
    assert_eq!(store.get_state_snapshot_before(1).unwrap(), Some((0, hash)));
    assert_eq!(store.get_state_snapshot_before(2).unwrap(), Some((0, hash)));

    // hack: VersionData expected on every version, so duplicate the data at version 1
    let usage = store.get_usage(Some(0)).unwrap();
    store
        .ledger_db
        .put::<VersionDataSchema>(&1, &usage.into())
        .unwrap();

    // put in another version
    put_value_set(store, kv, 2, Some(0));
    assert_eq!(store.get_state_snapshot_before(4).unwrap(), Some((2, hash)));
    assert_eq!(store.get_state_snapshot_before(3).unwrap(), Some((2, hash)));
    assert_eq!(store.get_state_snapshot_before(2).unwrap(), Some((0, hash)));
    assert_eq!(store.get_state_snapshot_before(1).unwrap(), Some((0, hash)));
    assert_eq!(store.get_state_snapshot_before(0).unwrap(), None,);
}

// Initializes the state store by inserting one key at each version.
fn init_store(store: &StateStore, input: impl Iterator<Item = (StateKey, StateValue)>) {
    update_store(store, input.into_iter().map(|(k, v)| (k, Some(v))), 0);
}
