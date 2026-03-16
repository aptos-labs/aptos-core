// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::AptosDB,
    schema::hot_state_value_by_key_hash::{HotStateKvValue, HotStateValueByKeyHashSchema},
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

#[test]
fn test_basic_insertion_write_read() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key = make_state_key(1);
    let value = make_state_value(1);
    let hot_since_version = 10;
    let value_version = 5;
    let shard_id = key.get_shard_id();

    let entry = HotStateKvValue::Occupied {
        value_version,
        value: value.clone(),
    };

    // Write
    let mut batch = db.db_shard(shard_id).new_native_batch();
    batch
        .put::<HotStateValueByKeyHashSchema>(
            &(CryptoHash::hash(&key), hot_since_version),
            &Some(entry),
        )
        .unwrap();
    db.db_shard(shard_id).write_schemas(batch).unwrap();

    // Read back
    let result = db
        .db_shard(shard_id)
        .get::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(&key), hot_since_version))
        .unwrap()
        .unwrap();

    assert_eq!(
        result,
        Some(HotStateKvValue::Occupied {
            value_version,
            value,
        })
    );
}

#[test]
fn test_vacant_insertion_write_read() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key = make_state_key(2);
    let hot_since_version: Version = 20;
    let shard_id = key.get_shard_id();

    let entry = HotStateKvValue::Vacant;

    let mut batch = db.db_shard(shard_id).new_native_batch();
    batch
        .put::<HotStateValueByKeyHashSchema>(
            &(CryptoHash::hash(&key), hot_since_version),
            &Some(entry),
        )
        .unwrap();
    db.db_shard(shard_id).write_schemas(batch).unwrap();

    let result = db
        .db_shard(shard_id)
        .get::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(&key), hot_since_version))
        .unwrap()
        .unwrap();

    let read_entry = result.unwrap();
    assert_eq!(read_entry, HotStateKvValue::Vacant);
}

#[test]
fn test_eviction_write_read() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key = make_state_key(3);
    let shard_id = key.get_shard_id();
    let key_hash = CryptoHash::hash(&key);

    // Write insertion at V10
    let entry = HotStateKvValue::Occupied {
        value_version: 5,
        value: make_state_value(3),
    };
    let mut batch = db.db_shard(shard_id).new_native_batch();
    batch
        .put::<HotStateValueByKeyHashSchema>(&(key_hash, 10), &Some(entry))
        .unwrap();
    db.db_shard(shard_id).write_schemas(batch).unwrap();

    // Write eviction at V20
    let mut batch = db.db_shard(shard_id).new_native_batch();
    batch
        .put::<HotStateValueByKeyHashSchema>(&(key_hash, 20), &None::<HotStateKvValue>)
        .unwrap();
    db.db_shard(shard_id).write_schemas(batch).unwrap();

    // The latest entry (V20, descending) should be None (eviction).
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
    let shard_id = key.get_shard_id();
    let key_hash = CryptoHash::hash(&key);

    // Insertion at V1
    let entry = HotStateKvValue::Occupied {
        value_version: 1,
        value: make_state_value(41),
    };

    let mut batch = db.db_shard(shard_id).new_native_batch();
    batch
        .put::<HotStateValueByKeyHashSchema>(&(key_hash, 1), &Some(entry.clone()))
        .unwrap();
    // Refresh at V2 (new hot_since_version, same value_version and value)
    batch
        .put::<HotStateValueByKeyHashSchema>(&(key_hash, 2), &Some(entry))
        .unwrap();
    db.db_shard(shard_id).write_schemas(batch).unwrap();

    // Querying at Version::MAX should return the latest (V2).
    let (latest_version, latest_entry) = db
        .get_hot_state_entry_by_version(&key, Version::MAX)
        .unwrap()
        .unwrap();
    assert_eq!(latest_version, 2);
    assert!(latest_entry.is_some());

    // Querying at V1 should return the older entry.
    let (older_version, older_entry) = db.get_hot_state_entry_by_version(&key, 1).unwrap().unwrap();
    assert_eq!(older_version, 1);
    assert!(older_entry.is_some());
}

#[test]
fn test_cross_shard() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    // Create keys that land in different shards
    let mut shard_keys: Vec<Option<StateKey>> = vec![None; NUM_STATE_SHARDS];
    for i in 0u16..1000 {
        let key = StateKey::raw(&i.to_be_bytes());
        let shard = key.get_shard_id();
        if shard_keys[shard].is_none() {
            shard_keys[shard] = Some(key);
        }
        if shard_keys.iter().all(|k| k.is_some()) {
            break;
        }
    }

    // Write one entry per shard
    for (shard_id, key_opt) in shard_keys.iter().enumerate() {
        let key = key_opt
            .as_ref()
            .unwrap_or_else(|| panic!("No key found for shard {shard_id}"));
        assert_eq!(key.get_shard_id(), shard_id);

        let entry = HotStateKvValue::Occupied {
            value_version: 100,
            value: make_state_value(shard_id as u8),
        };

        let mut batch = db.db_shard(shard_id).new_native_batch();
        batch
            .put::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(key), 100), &Some(entry))
            .unwrap();
        db.db_shard(shard_id).write_schemas(batch).unwrap();
    }

    // Read back and verify each shard has exactly one correct entry
    for (shard_id, key_opt) in shard_keys.iter().enumerate() {
        let key = key_opt.as_ref().unwrap();
        let result = db
            .db_shard(shard_id)
            .get::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(key), 100))
            .unwrap()
            .unwrap();
        assert_eq!(result.unwrap(), HotStateKvValue::Occupied {
            value_version: 100,
            value: make_state_value(shard_id as u8),
        });
    }
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

    let shard1 = key1.get_shard_id();
    let shard2 = key2.get_shard_id();
    let shard3 = key3.get_shard_id();

    // Accumulate insertions and evictions per-shard, handling potential shard collisions.
    let mut per_shard_insertions: Vec<HashMap<StateKey, (HotStateValue, Option<Version>)>> =
        (0..NUM_STATE_SHARDS).map(|_| HashMap::new()).collect();
    let mut per_shard_evictions: Vec<HashMap<StateKey, Version>> =
        (0..NUM_STATE_SHARDS).map(|_| HashMap::new()).collect();

    // key1: occupied insertion at hot_since_version=100, value_version=50
    per_shard_insertions[shard1].insert(
        key1.clone(),
        (HotStateValue::new(Some(val1.clone()), 100), Some(50)),
    );

    // key2: vacant insertion at hot_since_version=200
    per_shard_insertions[shard2].insert(key2.clone(), (HotStateValue::new(None, 200), None));

    // key3: eviction at version=300
    per_shard_evictions[shard3].insert(key3.clone(), 300_u64);

    let shards: [HotStateShardUpdates; NUM_STATE_SHARDS] = per_shard_insertions
        .into_iter()
        .zip(per_shard_evictions)
        .map(|(ins, ev)| HotStateShardUpdates::new(ins, ev))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

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
        hot_state_kv_db
            .db_shard(shard1)
            .get::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(&key1), 100))
            .unwrap()
            .unwrap()
            .unwrap(),
        HotStateKvValue::Occupied {
            value_version: 50,
            value: val1,
        }
    );

    // Verify key2: vacant
    assert_eq!(
        hot_state_kv_db
            .db_shard(shard2)
            .get::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(&key2), 200))
            .unwrap()
            .unwrap()
            .unwrap(),
        HotStateKvValue::Vacant,
    );

    // Verify key3: eviction (None)
    assert!(
        hot_state_kv_db
            .db_shard(shard3)
            .get::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(&key3), 300))
            .unwrap()
            .unwrap()
            .is_none(),
        "Eviction should be None"
    );
}
