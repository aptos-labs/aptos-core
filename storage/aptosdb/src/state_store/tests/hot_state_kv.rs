// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    schema::hot_state_value_by_key_hash::{
        HotStateKvEntry, HotStateKvValue, HotStateValueByKeyHashSchema,
    },
    state_kv_db::StateKvDb,
};
use aptos_config::config::{RocksdbConfig, StorageDirPaths};
use aptos_crypto::hash::CryptoHash;
use aptos_schemadb::{batch::WriteBatch, ReadOptions};
use aptos_temppath::TempPath;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue, NUM_STATE_SHARDS},
    transaction::Version,
};

fn create_hot_state_kv_db(path: &TempPath) -> StateKvDb {
    StateKvDb::new(
        &StorageDirPaths::from_path(path.path()),
        RocksdbConfig::default(),
        None,
        None,
        /* readonly = */ false,
        /* is_hot = */ true,
        /* delete_on_restart = */ true,
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
    let hot_since_version: Version = 10;
    let value_version: Version = 5;
    let shard_id = key.get_shard_id();

    let entry = HotStateKvEntry {
        state_key: key.clone(),
        value: HotStateKvValue::Occupied {
            value_version,
            value: value.clone(),
        },
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

    let read_entry = result.unwrap();
    assert_eq!(read_entry.state_key, key);
    match &read_entry.value {
        HotStateKvValue::Occupied {
            value_version: vv,
            value: v,
        } => {
            assert_eq!(*vv, value_version);
            assert_eq!(*v, value);
        },
        HotStateKvValue::Vacant => panic!("Expected Occupied"),
    }
}

#[test]
fn test_vacant_insertion_write_read() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key = make_state_key(2);
    let hot_since_version: Version = 20;
    let shard_id = key.get_shard_id();

    let entry = HotStateKvEntry {
        state_key: key.clone(),
        value: HotStateKvValue::Vacant,
    };

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
    assert_eq!(read_entry.state_key, key);
    assert_eq!(read_entry.value, HotStateKvValue::Vacant);
}

#[test]
fn test_eviction_write_read() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key = make_state_key(3);
    let shard_id = key.get_shard_id();
    let key_hash = CryptoHash::hash(&key);

    // Write insertion at V10
    let entry = HotStateKvEntry {
        state_key: key.clone(),
        value: HotStateKvValue::Occupied {
            value_version: 5,
            value: make_state_value(3),
        },
    };
    let mut batch = db.db_shard(shard_id).new_native_batch();
    batch
        .put::<HotStateValueByKeyHashSchema>(&(key_hash, 10), &Some(entry))
        .unwrap();
    db.db_shard(shard_id).write_schemas(batch).unwrap();

    // Write eviction at V20
    let mut batch = db.db_shard(shard_id).new_native_batch();
    batch
        .put::<HotStateValueByKeyHashSchema>(&(key_hash, 20), &None)
        .unwrap();
    db.db_shard(shard_id).write_schemas(batch).unwrap();

    // Seek to key_hash — the latest entry (V20, descending) should be None (eviction).
    let mut read_opts = ReadOptions::default();
    read_opts.set_prefix_same_as_start(true);
    let mut iter = db
        .db_shard(shard_id)
        .iter_with_opts::<HotStateValueByKeyHashSchema>(read_opts)
        .unwrap();
    iter.seek(&(key_hash, Version::MAX)).unwrap();

    let ((found_hash, found_version), found_value) = iter.next().unwrap().unwrap();
    assert_eq!(found_hash, key_hash);
    // Latest version (descending order means higher version first)
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
    let entry_v1 = HotStateKvEntry {
        state_key: key.clone(),
        value: HotStateKvValue::Occupied {
            value_version: 1,
            value: make_state_value(41),
        },
    };
    // Refresh at V2 (new hot_since_version, same value_version)
    let entry_v2 = HotStateKvEntry {
        state_key: key.clone(),
        value: HotStateKvValue::Occupied {
            value_version: 1,
            value: make_state_value(41),
        },
    };

    let mut batch = db.db_shard(shard_id).new_native_batch();
    batch
        .put::<HotStateValueByKeyHashSchema>(&(key_hash, 1), &Some(entry_v1))
        .unwrap();
    batch
        .put::<HotStateValueByKeyHashSchema>(&(key_hash, 2), &Some(entry_v2))
        .unwrap();
    db.db_shard(shard_id).write_schemas(batch).unwrap();

    // Iterate by prefix — V2 should come first (descending), then V1.
    let mut read_opts = ReadOptions::default();
    read_opts.set_prefix_same_as_start(true);
    let mut iter = db
        .db_shard(shard_id)
        .iter_with_opts::<HotStateValueByKeyHashSchema>(read_opts)
        .unwrap();
    iter.seek(&(key_hash, Version::MAX)).unwrap();

    let ((_, v1), _) = iter.next().unwrap().unwrap();
    assert_eq!(v1, 2, "V2 should come first (descending)");

    let ((_, v2), _) = iter.next().unwrap().unwrap();
    assert_eq!(v2, 1, "V1 should come second");

    assert!(iter.next().is_none(), "No more entries");
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

        let entry = HotStateKvEntry {
            state_key: key.clone(),
            value: HotStateKvValue::Occupied {
                value_version: 100,
                value: make_state_value(shard_id as u8),
            },
        };

        let mut batch = db.db_shard(shard_id).new_native_batch();
        batch
            .put::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(key), 100), &Some(entry))
            .unwrap();
        db.db_shard(shard_id).write_schemas(batch).unwrap();
    }

    // Read back and verify each shard has exactly one entry
    for (shard_id, key_opt) in shard_keys.iter().enumerate() {
        let key = key_opt.as_ref().unwrap();
        let result = db
            .db_shard(shard_id)
            .get::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(key), 100))
            .unwrap()
            .unwrap();
        let read_entry = result.unwrap();
        assert_eq!(read_entry.state_key, *key);
    }
}

#[test]
fn test_put_hot_state_kv_updates_integration() {
    use aptos_storage_interface::state_store::HotStateShardUpdates;
    use aptos_types::state_store::hot_state::HotStateValue;
    use std::collections::HashMap;

    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

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

    // Write using the same logic as put_hot_state_kv_updates
    let mut sharded_batches = db.new_sharded_native_batches();
    for (batch, shard) in sharded_batches.iter_mut().zip(shards.iter()) {
        for (key, (hot_val, value_version_opt)) in shard.insertions() {
            let schema_value = match value_version_opt {
                Some(vv) => HotStateKvValue::Occupied {
                    value_version: *vv,
                    value: hot_val.value().expect("occupied must have value").clone(),
                },
                None => HotStateKvValue::Vacant,
            };
            batch
                .put::<HotStateValueByKeyHashSchema>(
                    &(CryptoHash::hash(key), hot_val.hot_since_version()),
                    &Some(HotStateKvEntry {
                        state_key: key.clone(),
                        value: schema_value,
                    }),
                )
                .unwrap();
        }
        for (key, eviction_version) in shard.evictions() {
            batch
                .put::<HotStateValueByKeyHashSchema>(
                    &(CryptoHash::hash(key), *eviction_version),
                    &None,
                )
                .unwrap();
        }
    }

    db.commit(999, None, sharded_batches).unwrap();

    // Verify key1: occupied
    let result = db
        .db_shard(shard1)
        .get::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(&key1), 100))
        .unwrap()
        .unwrap();
    let entry = result.unwrap();
    assert_eq!(entry.state_key, key1);
    match entry.value {
        HotStateKvValue::Occupied {
            value_version,
            value,
        } => {
            assert_eq!(value_version, 50);
            assert_eq!(value, val1);
        },
        HotStateKvValue::Vacant => panic!("Expected Occupied for key1"),
    }

    // Verify key2: vacant
    let result = db
        .db_shard(shard2)
        .get::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(&key2), 200))
        .unwrap()
        .unwrap();
    let entry = result.unwrap();
    assert_eq!(entry.state_key, key2);
    assert_eq!(entry.value, HotStateKvValue::Vacant);

    // Verify key3: eviction (None)
    let result = db
        .db_shard(shard3)
        .get::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(&key3), 300))
        .unwrap()
        .unwrap();
    assert!(result.is_none(), "Eviction should be None");
}
