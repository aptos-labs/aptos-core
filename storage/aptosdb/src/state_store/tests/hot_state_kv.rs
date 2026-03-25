// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::AptosDB,
    schema::hot_state_value_by_key_hash::{HotStateEntry, HotStateValueByKeyHashSchema},
    state_kv_db::StateKvDb,
};
use aptos_config::config::{RocksdbConfig, StorageDirPaths};
use aptos_crypto::hash::CryptoHash;
use aptos_schemadb::batch::WriteBatch;
use aptos_storage_interface::state_store::{HotStateShardUpdates, HotStateUpdates};
use aptos_temppath::TempPath;
use aptos_types::{
    state_store::{
        hot_state::HotStateValue, state_key::StateKey, state_value::StateValue, NUM_STATE_SHARDS,
    },
    transaction::Version,
};
use std::collections::HashMap;

fn create_hot_state_kv_db(path: &TempPath) -> StateKvDb {
    StateKvDb::new(
        &StorageDirPaths::from_path(path.path()),
        RocksdbConfig::default(),
        /* env = */ None,
        /* block_cache = */ None,
        /* readonly = */ false,
        /* is_hot = */ true,
        /* delete_on_restart = */ false,
    )
    .unwrap()
}

fn make_state_key(seed: u8) -> StateKey {
    StateKey::raw(&[seed])
}

fn make_state_value(seed: u8) -> StateValue {
    StateValue::new_legacy(vec![seed, seed].into())
}

fn put_hot_state_entry(
    db: &StateKvDb,
    key: &StateKey,
    version: Version,
    entry: Option<HotStateEntry>,
) {
    let shard_id = key.get_shard_id();
    let mut batch = db.db_shard(shard_id).new_native_batch();
    batch
        .put::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(key), version), &entry)
        .unwrap();
    db.db_shard(shard_id).write_schemas(batch).unwrap();
}

fn get_hot_state_entry(db: &StateKvDb, key: &StateKey, version: Version) -> Option<HotStateEntry> {
    db.db_shard(key.get_shard_id())
        .get::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(key), version))
        .unwrap()
        .unwrap()
}

#[test]
fn test_insertion_write_read() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    // Occupied entry
    let key1 = make_state_key(1);
    let value = make_state_value(1);
    put_hot_state_entry(
        &db,
        &key1,
        10,
        Some(HotStateEntry::Occupied {
            value: value.clone(),
            value_version: 5,
        }),
    );
    assert_eq!(
        get_hot_state_entry(&db, &key1, 10),
        Some(HotStateEntry::Occupied {
            value,
            value_version: 5,
        })
    );

    // Vacant entry
    let key2 = make_state_key(2);
    put_hot_state_entry(&db, &key2, 20, Some(HotStateEntry::Vacant));
    assert_eq!(
        get_hot_state_entry(&db, &key2, 20),
        Some(HotStateEntry::Vacant),
    );
}

#[test]
fn test_eviction_write_read() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key = make_state_key(3);

    // Insertion at V10, eviction at V20.
    put_hot_state_entry(
        &db,
        &key,
        10,
        Some(HotStateEntry::Occupied {
            value: make_state_value(3),
            value_version: 5,
        }),
    );
    put_hot_state_entry(&db, &key, 20, None);

    // The latest entry should be the eviction.
    let (found_version, found_value) = db
        .get_hot_state_entry_by_version(&key, Version::MAX)
        .unwrap()
        .unwrap();
    assert_eq!(found_version, 20);
    assert!(found_value.is_none(), "Eviction should be None");
}

#[test]
fn test_multiple_versions() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key = make_state_key(4);
    let entry = HotStateEntry::Occupied {
        value: make_state_value(41),
        value_version: 1,
    };

    // Insertion at V1, refresh at V2.
    put_hot_state_entry(&db, &key, 1, Some(entry.clone()));
    put_hot_state_entry(&db, &key, 2, Some(entry));

    let expected_entry = Some(HotStateEntry::Occupied {
        value: make_state_value(41),
        value_version: 1,
    });

    // Querying at Version::MAX should return the latest (V2).
    let (latest_version, latest_entry) = db
        .get_hot_state_entry_by_version(&key, Version::MAX)
        .unwrap()
        .unwrap();
    assert_eq!(latest_version, 2);
    assert_eq!(latest_entry, expected_entry);

    // Querying at V1 should return the older entry.
    let (older_version, older_entry) = db.get_hot_state_entry_by_version(&key, 1).unwrap().unwrap();
    assert_eq!(older_version, 1);
    assert_eq!(older_entry, expected_entry);
}

#[test]
fn test_put_hot_state_updates_integration() {
    let tmp = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp);
    let hot_state_kv_db = aptos_db.hot_state_kv_db.as_ref().unwrap();

    let key1 = make_state_key(10);
    let val1 = make_state_value(10);
    let key2 = make_state_key(20);
    let key3 = make_state_key(30);

    let mut shards: [HotStateShardUpdates; NUM_STATE_SHARDS] =
        std::array::from_fn(|_| HotStateShardUpdates::new(HashMap::new(), HashMap::new()));

    // key1: occupied insertion at hot_since_version=100, value_version=50
    shards[key1.get_shard_id()].insertions.insert(
        *key1.crypto_hash_ref(),
        (HotStateValue::new(Some(val1.clone()), 100), Some(50)),
    );

    // key2: vacant insertion at hot_since_version=200
    shards[key2.get_shard_id()].insertions.insert(
        *key2.crypto_hash_ref(),
        (HotStateValue::new(None, 200), None),
    );

    // key3: eviction at version=300
    shards[key3.get_shard_id()]
        .evictions
        .insert(*key3.crypto_hash_ref(), 300);

    let hot_state_updates = HotStateUpdates {
        for_last_checkpoint: Some(shards),
        for_latest: None,
    };

    let mut sharded_batches = hot_state_kv_db.new_sharded_native_batches();
    aptos_db
        .state_store
        .put_hot_state_updates(&hot_state_updates, &mut sharded_batches)
        .unwrap();
    hot_state_kv_db.commit(999, None, sharded_batches).unwrap();

    // Verify key1: occupied
    assert_eq!(
        get_hot_state_entry(hot_state_kv_db, &key1, 100).unwrap(),
        HotStateEntry::Occupied {
            value: val1,
            value_version: 50,
        }
    );

    // Verify key2: vacant
    assert_eq!(
        get_hot_state_entry(hot_state_kv_db, &key2, 200).unwrap(),
        HotStateEntry::Vacant,
    );

    // Verify key3: eviction (None)
    assert!(
        get_hot_state_entry(hot_state_kv_db, &key3, 300).is_none(),
        "Eviction should be None"
    );
}
