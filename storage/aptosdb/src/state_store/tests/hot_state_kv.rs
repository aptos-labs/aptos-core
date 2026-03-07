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
    state_store::{
        state_key::StateKey, state_slot::StateSlot, state_value::StateValue, NUM_STATE_SHARDS,
    },
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

// ===== Loading tests =====

fn create_persistent_hot_state_kv_db(path: &TempPath) -> StateKvDb {
    StateKvDb::new(
        &StorageDirPaths::from_path(path.path()),
        RocksdbConfig::default(),
        None,
        None,
        /* readonly = */ false,
        /* is_hot = */ true,
        /* delete_on_restart = */ false,
    )
    .unwrap()
}

/// Write a hot state entry to the DB directly.
fn write_entry(
    db: &StateKvDb,
    key: &StateKey,
    hot_since_version: Version,
    value: &HotStateKvValue,
) {
    let shard_id = key.get_shard_id();
    let entry = HotStateKvEntry {
        state_key: key.clone(),
        value: value.clone(),
    };
    let mut batch = db.db_shard(shard_id).new_native_batch();
    batch
        .put::<HotStateValueByKeyHashSchema>(
            &(CryptoHash::hash(key), hot_since_version),
            &Some(entry),
        )
        .unwrap();
    db.db_shard(shard_id).write_schemas(batch).unwrap();
}

/// Write an eviction marker to the DB.
fn write_eviction(db: &StateKvDb, key: &StateKey, eviction_version: Version) {
    let shard_id = key.get_shard_id();
    let mut batch = db.db_shard(shard_id).new_native_batch();
    batch
        .put::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(key), eviction_version), &None)
        .unwrap();
    db.db_shard(shard_id).write_schemas(batch).unwrap();
}

#[test]
fn test_load_empty_db() {
    let tmp = TempPath::new();
    let db = create_persistent_hot_state_kv_db(&tmp);

    let entries = db.load_all_hot_state_entries().unwrap();
    for shard in &entries {
        assert!(shard.is_empty());
    }
}

#[test]
fn test_load_round_trip() {
    let tmp = TempPath::new();
    let db = create_persistent_hot_state_kv_db(&tmp);

    let key1 = make_state_key(1);
    let val1 = make_state_value(1);
    let key2 = make_state_key(2);

    // Insert key1: occupied at hot_since_version=10, value_version=5
    write_entry(&db, &key1, 10, &HotStateKvValue::Occupied {
        value_version: 5,
        value: val1.clone(),
    });

    // Insert key2: vacant at hot_since_version=20
    write_entry(&db, &key2, 20, &HotStateKvValue::Vacant);

    let entries = db.load_all_hot_state_entries().unwrap();

    // Find key1
    let shard1 = key1.get_shard_id();
    let found1 = entries[shard1]
        .iter()
        .find(|(k, _)| k == &key1)
        .expect("key1 should be loaded");
    match &found1.1 {
        StateSlot::HotOccupied {
            value_version,
            value,
            hot_since_version,
            ..
        } => {
            assert_eq!(*value_version, 5);
            assert_eq!(*value, val1);
            assert_eq!(*hot_since_version, 10);
        },
        _ => panic!("Expected HotOccupied for key1"),
    }

    // Find key2
    let shard2 = key2.get_shard_id();
    let found2 = entries[shard2]
        .iter()
        .find(|(k, _)| k == &key2)
        .expect("key2 should be loaded");
    match &found2.1 {
        StateSlot::HotVacant {
            hot_since_version, ..
        } => {
            assert_eq!(*hot_since_version, 20);
        },
        _ => panic!("Expected HotVacant for key2"),
    }
}

#[test]
fn test_load_eviction_filtering() {
    let tmp = TempPath::new();
    let db = create_persistent_hot_state_kv_db(&tmp);

    let key = make_state_key(10);

    // Insert at V10, evict at V20
    write_entry(&db, &key, 10, &HotStateKvValue::Occupied {
        value_version: 5,
        value: make_state_value(10),
    });
    write_eviction(&db, &key, 20);

    let entries = db.load_all_hot_state_entries().unwrap();
    let shard = key.get_shard_id();
    assert!(
        !entries[shard].iter().any(|(k, _)| k == &key),
        "Evicted key should not be present"
    );
}

#[test]
fn test_load_reinsertion_after_eviction() {
    let tmp = TempPath::new();
    let db = create_persistent_hot_state_kv_db(&tmp);

    let key = make_state_key(11);
    let val_v3 = make_state_value(33);

    // Insert at V10
    write_entry(&db, &key, 10, &HotStateKvValue::Occupied {
        value_version: 5,
        value: make_state_value(11),
    });
    // Evict at V20
    write_eviction(&db, &key, 20);
    // Re-insert at V30
    write_entry(&db, &key, 30, &HotStateKvValue::Occupied {
        value_version: 25,
        value: val_v3.clone(),
    });

    let entries = db.load_all_hot_state_entries().unwrap();
    let shard = key.get_shard_id();
    let found = entries[shard]
        .iter()
        .find(|(k, _)| k == &key)
        .expect("Re-inserted key should be present");

    match &found.1 {
        StateSlot::HotOccupied {
            value_version,
            value,
            hot_since_version,
            ..
        } => {
            assert_eq!(*hot_since_version, 30);
            assert_eq!(*value_version, 25);
            assert_eq!(*value, val_v3);
        },
        _ => panic!("Expected HotOccupied"),
    }
}

#[test]
fn test_load_lru_reconstruction() {
    use aptos_types::state_store::hot_state::THotStateSlot;

    let tmp = TempPath::new();
    let db = create_persistent_hot_state_kv_db(&tmp);

    // Create keys that all land in the same shard for easier testing.
    // Try seeds until we find 4 that share a shard.
    let mut same_shard_keys = Vec::new();
    let target_shard = make_state_key(0).get_shard_id();
    for seed in 0u8..=255 {
        let key = make_state_key(seed);
        if key.get_shard_id() == target_shard {
            same_shard_keys.push((seed, key));
        }
        if same_shard_keys.len() == 4 {
            break;
        }
    }
    assert!(same_shard_keys.len() == 4, "Need 4 keys in the same shard");

    // Insert with hot_since_versions: 10, 5, 20, 15
    let versions = [10u64, 5, 20, 15];
    for (i, &(seed, ref key)) in same_shard_keys.iter().enumerate() {
        write_entry(&db, key, versions[i], &HotStateKvValue::Occupied {
            value_version: versions[i],
            value: make_state_value(seed),
        });
    }

    let entries = db.load_all_hot_state_entries().unwrap();
    let shard_entries = &entries[target_shard];

    // Should be sorted by hot_since_version ascending: 5, 10, 15, 20
    let hsv: Vec<Version> = shard_entries
        .iter()
        .map(|(_, slot)| slot.expect_hot_since_version())
        .collect();
    assert_eq!(hsv, vec![5, 10, 15, 20], "Should be sorted ascending");

    // Verify LRU pointers.
    // Index 0 (oldest, hsv=5): prev=Some(key@hsv=10), next=None
    // Index 1 (hsv=10):        prev=Some(key@hsv=15), next=Some(key@hsv=5)
    // Index 2 (hsv=15):        prev=Some(key@hsv=20), next=Some(key@hsv=10)
    // Index 3 (newest, hsv=20):prev=None,             next=Some(key@hsv=15)
    let len = shard_entries.len();
    for i in 0..len {
        let (_, slot) = &shard_entries[i];
        if i == len - 1 {
            // Newest — no prev (no newer entry)
            assert!(slot.prev().is_none(), "Newest should have no prev");
        } else {
            assert_eq!(
                slot.prev(),
                Some(&shard_entries[i + 1].0),
                "prev should point to entry at index {}",
                i + 1
            );
        }
        if i == 0 {
            // Oldest — no next (no older entry)
            assert!(slot.next().is_none(), "Oldest should have no next");
        } else {
            assert_eq!(
                slot.next(),
                Some(&shard_entries[i - 1].0),
                "next should point to entry at index {}",
                i - 1
            );
        }
    }
}

#[test]
fn test_load_only_latest_version_per_key() {
    let tmp = TempPath::new();
    let db = create_persistent_hot_state_kv_db(&tmp);

    let key = make_state_key(50);
    let val_old = make_state_value(50);
    let val_new = make_state_value(51);

    // Insert at V1 then refresh at V2
    write_entry(&db, &key, 1, &HotStateKvValue::Occupied {
        value_version: 1,
        value: val_old,
    });
    write_entry(&db, &key, 2, &HotStateKvValue::Occupied {
        value_version: 1,
        value: val_new.clone(),
    });

    let entries = db.load_all_hot_state_entries().unwrap();
    let shard = key.get_shard_id();

    // Should have exactly one entry for this key, at hot_since_version=2
    let matches: Vec<_> = entries[shard].iter().filter(|(k, _)| k == &key).collect();
    assert_eq!(matches.len(), 1, "Should deduplicate to latest version");
    assert_eq!(matches[0].1.expect_hot_since_version(), 2);
    match &matches[0].1 {
        StateSlot::HotOccupied { value, .. } => {
            assert_eq!(*value, val_new);
        },
        _ => panic!("Expected HotOccupied"),
    }
}

#[test]
fn test_hot_state_from_loaded_entries() {
    use crate::state_store::hot_state::HotState;
    use aptos_config::config::HotStateConfig;
    use aptos_storage_interface::state_store::state::State;

    let tmp = TempPath::new();
    let db = create_persistent_hot_state_kv_db(&tmp);

    let key1 = make_state_key(100);
    let val1 = make_state_value(100);
    let key2 = make_state_key(200);

    write_entry(&db, &key1, 10, &HotStateKvValue::Occupied {
        value_version: 5,
        value: val1.clone(),
    });
    write_entry(&db, &key2, 20, &HotStateKvValue::Vacant);

    let entries = db.load_all_hot_state_entries().unwrap();

    let config = HotStateConfig {
        max_items_per_shard: 1000,
        refresh_interval_versions: 100,
        delete_on_restart: false,
        compute_root_hash: false,
    };

    // Clone entries for HotState (State takes ownership conceptually, HotState needs a copy)
    let entries_for_hot_state: [Vec<(StateKey, StateSlot)>; NUM_STATE_SHARDS] =
        std::array::from_fn(|i| entries[i].clone());

    let state = State::new_from_hot_state_entries(Some(20), entries, config);
    assert_eq!(state.next_version(), 21);

    let hot_state = HotState::new_with_base(state, config, entries_for_hot_state);
    let (view, committed_state) = hot_state.get_committed();
    assert_eq!(committed_state.next_version(), 21);

    // key1 should be found via the HotStateView
    let slot1 = view.get_state_slot(&key1);
    assert!(slot1.is_some(), "key1 should be in hot state");
    let slot1 = slot1.unwrap();
    assert!(slot1.is_hot());
    assert_eq!(slot1.as_state_value_opt(), Some(&val1));

    // key2 should be found as HotVacant
    let slot2 = view.get_state_slot(&key2);
    assert!(slot2.is_some(), "key2 should be in hot state");
    let slot2 = slot2.unwrap();
    assert!(slot2.is_hot());
    assert!(slot2.as_state_value_opt().is_none());

    // unknown key should not be found
    let unknown = make_state_key(255);
    assert!(view.get_state_slot(&unknown).is_none());
}
