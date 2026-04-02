// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::AptosDB,
    schema::hot_state_value_by_key_hash::{HotStateEntry, HotStateValueByKeyHashSchema},
    state_kv_db::{LoadedHotStateShard, StateKvDb},
};
use aptos_config::config::{RocksdbConfig, StorageDirPaths};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_schemadb::batch::WriteBatch;
use aptos_storage_interface::state_store::{HotStateShardUpdates, HotStateUpdates};
use aptos_temppath::TempPath;
use aptos_types::{
    state_store::{
        hot_state::{HotStateValue, THotStateSlot},
        state_key::StateKey,
        state_slot::StateSlotKind,
        state_value::StateValue,
        NUM_STATE_SHARDS,
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

// ---------------------------------------------------------------------------
// load_hot_state_kvs tests
// ---------------------------------------------------------------------------

/// Collect the LRU chain from head→tail as a list of (key_hash, hot_since_version).
fn collect_lru_order(shard: &LoadedHotStateShard) -> Vec<(HashValue, Version)> {
    let mut result = Vec::new();
    let mut current = shard.head;
    while let Some(kh) = current {
        let slot = shard.map.get(&kh).unwrap();
        let hsv = slot.expect_hot_since_version();
        result.push((kh, hsv));
        current = slot.next().copied();
    }
    result
}

#[test]
fn test_load_empty_db() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let shards = db.load_hot_state_kvs(100).unwrap();
    for shard in &shards {
        assert_eq!(shard.num_items, 0);
        assert!(shard.head.is_none());
        assert!(shard.tail.is_none());
        assert!(shard.map.is_empty());
    }
}

#[test]
fn test_load_single_entry() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key = make_state_key(1);
    let key_hash = CryptoHash::hash(&key);
    put_hot_state_entry(
        &db,
        &key,
        10,
        Some(HotStateEntry::Occupied {
            value: make_state_value(1),
            value_version: 5,
        }),
    );

    let shards = db.load_hot_state_kvs(100).unwrap();
    let shard = &shards[key.get_shard_id()];
    assert_eq!(shard.num_items, 1);
    assert_eq!(shard.head, Some(key_hash));
    assert_eq!(shard.tail, Some(key_hash));

    let slot = shard.map.get(&key_hash).unwrap();
    assert!(slot.is_hot());
    assert!(slot.prev().is_none()); // Only entry: head.
    assert!(slot.next().is_none()); // Only entry: tail.
    match slot.kind() {
        StateSlotKind::HotOccupied {
            value_version,
            value,
            hot_since_version,
            ..
        } => {
            assert_eq!(*value_version, 5);
            assert_eq!(*value, make_state_value(1));
            assert_eq!(*hot_since_version, 10);
        },
        other => panic!("Expected HotOccupied, got {other:?}"),
    }
    shard.validate_lru_chain();
}

#[test]
fn test_load_occupied_and_vacant() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key_occ = make_state_key(10);
    put_hot_state_entry(
        &db,
        &key_occ,
        100,
        Some(HotStateEntry::Occupied {
            value: make_state_value(10),
            value_version: 50,
        }),
    );

    let key_vac = make_state_key(20);
    put_hot_state_entry(&db, &key_vac, 200, Some(HotStateEntry::Vacant));

    let shards = db.load_hot_state_kvs(300).unwrap();

    // Check occupied.
    let shard_occ = &shards[key_occ.get_shard_id()];
    let slot_occ = shard_occ.map.get(&CryptoHash::hash(&key_occ)).unwrap();
    assert!(matches!(slot_occ.kind(), StateSlotKind::HotOccupied { .. }));
    shard_occ.validate_lru_chain();

    // Check vacant.
    let shard_vac = &shards[key_vac.get_shard_id()];
    let slot_vac = shard_vac.map.get(&CryptoHash::hash(&key_vac)).unwrap();
    assert!(matches!(slot_vac.kind(), StateSlotKind::HotVacant { .. }));
    shard_vac.validate_lru_chain();
}

#[test]
fn test_load_evicted_excluded() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key = make_state_key(5);
    // Insert at V10, evict at V20.
    put_hot_state_entry(
        &db,
        &key,
        10,
        Some(HotStateEntry::Occupied {
            value: make_state_value(5),
            value_version: 3,
        }),
    );
    put_hot_state_entry(&db, &key, 20, None);

    let shards = db.load_hot_state_kvs(100).unwrap();
    let shard = &shards[key.get_shard_id()];
    // The key's most recent entry is an eviction — should not appear.
    assert!(!shard.map.contains_key(&CryptoHash::hash(&key)));
}

#[test]
fn test_load_at_exact_committed_version() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key = make_state_key(6);
    put_hot_state_entry(
        &db,
        &key,
        50,
        Some(HotStateEntry::Occupied {
            value: make_state_value(6),
            value_version: 40,
        }),
    );

    // Load at the committed version — entry should appear.
    let shards = db.load_hot_state_kvs(50).unwrap();
    assert!(shards[key.get_shard_id()]
        .map
        .contains_key(&CryptoHash::hash(&key)));
}

#[test]
fn test_load_multiple_versions_same_key() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key = make_state_key(7);
    // V10: occupied with value_version=5
    put_hot_state_entry(
        &db,
        &key,
        10,
        Some(HotStateEntry::Occupied {
            value: make_state_value(71),
            value_version: 5,
        }),
    );
    // V20: refresh — same value but new hot_since_version
    put_hot_state_entry(
        &db,
        &key,
        20,
        Some(HotStateEntry::Occupied {
            value: make_state_value(72),
            value_version: 15,
        }),
    );

    // Load at V20 — should pick V20 (the most recent entry).
    let shards = db.load_hot_state_kvs(20).unwrap();
    let shard = &shards[key.get_shard_id()];
    let slot = shard.map.get(&CryptoHash::hash(&key)).unwrap();
    match slot.kind() {
        StateSlotKind::HotOccupied {
            value_version,
            hot_since_version,
            value,
            ..
        } => {
            assert_eq!(*hot_since_version, 20);
            assert_eq!(*value_version, 15);
            assert_eq!(*value, make_state_value(72));
        },
        other => panic!("Expected HotOccupied, got {other:?}"),
    }
    shard.validate_lru_chain();
}

#[test]
fn test_load_evict_then_reinsert() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key = make_state_key(8);
    // V10: insert.
    put_hot_state_entry(
        &db,
        &key,
        10,
        Some(HotStateEntry::Occupied {
            value: make_state_value(81),
            value_version: 5,
        }),
    );
    // V20: evict.
    put_hot_state_entry(&db, &key, 20, None);
    // V30: re-insert.
    put_hot_state_entry(
        &db,
        &key,
        30,
        Some(HotStateEntry::Occupied {
            value: make_state_value(82),
            value_version: 25,
        }),
    );

    // Load at V30 — should pick the re-insertion (most recent entry).
    let shards = db.load_hot_state_kvs(30).unwrap();
    let shard = &shards[key.get_shard_id()];
    let slot = shard.map.get(&CryptoHash::hash(&key)).unwrap();
    match slot.kind() {
        StateSlotKind::HotOccupied {
            hot_since_version,
            value,
            ..
        } => {
            assert_eq!(*hot_since_version, 30);
            assert_eq!(*value, make_state_value(82));
        },
        other => panic!("Expected HotOccupied, got {other:?}"),
    }
    shard.validate_lru_chain();
}

#[test]
fn test_load_lru_chain_ordering() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    // Insert several keys with distinct hot_since_versions into the same shard.
    // We use seeds that happen to hash to the same shard.
    // Since we can't predict shards, insert many keys and test whichever shard has > 1 entry.
    let mut keys_by_shard: Vec<Vec<(StateKey, Version)>> =
        (0..NUM_STATE_SHARDS).map(|_| Vec::new()).collect();

    for seed in 0..50u8 {
        let key = make_state_key(seed);
        let shard_id = key.get_shard_id();
        let hot_since = (seed as u64 + 1) * 10;
        put_hot_state_entry(
            &db,
            &key,
            hot_since,
            Some(HotStateEntry::Occupied {
                value: make_state_value(seed),
                value_version: hot_since.saturating_sub(5),
            }),
        );
        keys_by_shard[shard_id].push((key, hot_since));
    }

    let shards = db.load_hot_state_kvs(1000).unwrap();

    for (shard_id, shard_keys) in keys_by_shard.iter().enumerate() {
        let shard = &shards[shard_id];
        assert_eq!(shard.num_items, shard_keys.len());
        shard.validate_lru_chain();

        if shard_keys.len() > 1 {
            // Verify ordering: head→tail should be descending hot_since_version.
            let chain = collect_lru_order(shard);
            for pair in chain.windows(2) {
                assert!(
                    (pair[0].1, pair[0].0) >= (pair[1].1, pair[1].0),
                    "LRU chain not in descending hot_since_version order: {:?} followed by {:?}",
                    pair[0],
                    pair[1],
                );
            }
        }
    }
}

#[test]
fn test_load_cross_shard() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    // Insert keys that go to different shards.
    let mut expected_per_shard: [usize; NUM_STATE_SHARDS] = [0; NUM_STATE_SHARDS];
    for seed in 0..32u8 {
        let key = make_state_key(seed);
        expected_per_shard[key.get_shard_id()] += 1;
        put_hot_state_entry(
            &db,
            &key,
            seed as u64 + 1,
            Some(HotStateEntry::Occupied {
                value: make_state_value(seed),
                value_version: 1,
            }),
        );
    }

    let shards = db.load_hot_state_kvs(100).unwrap();
    for (shard_id, expected) in expected_per_shard.iter().enumerate() {
        assert_eq!(
            shards[shard_id].num_items, *expected,
            "Shard {shard_id} item count mismatch"
        );
        shards[shard_id].validate_lru_chain();
    }
}

#[test]
fn test_load_write_then_load_roundtrip() {
    let tmp = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp);
    let hot_state_kv_db = aptos_db.hot_state_kv_db.as_ref().unwrap();

    let key1 = make_state_key(10);
    let val1 = make_state_value(10);
    let key2 = make_state_key(20);

    let mut shards: [HotStateShardUpdates; NUM_STATE_SHARDS] =
        std::array::from_fn(|_| HotStateShardUpdates::new(HashMap::new(), HashMap::new()));

    // key1: occupied at hot_since_version=100, value_version=50
    shards[key1.get_shard_id()].insertions.insert(
        *key1.crypto_hash_ref(),
        (HotStateValue::new(Some(val1.clone()), 100), Some(50)),
    );

    // key2: vacant at hot_since_version=200
    shards[key2.get_shard_id()].insertions.insert(
        *key2.crypto_hash_ref(),
        (HotStateValue::new(None, 200), None),
    );

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

    // Load back.
    let loaded_shards = hot_state_kv_db.load_hot_state_kvs(999).unwrap();

    // Verify key1.
    let shard1 = &loaded_shards[key1.get_shard_id()];
    let slot1 = shard1.map.get(&CryptoHash::hash(&key1)).unwrap();
    match slot1.kind() {
        StateSlotKind::HotOccupied {
            value_version,
            value,
            hot_since_version,
            ..
        } => {
            assert_eq!(*value_version, 50);
            assert_eq!(*value, val1);
            assert_eq!(*hot_since_version, 100);
        },
        other => panic!("Expected HotOccupied for key1, got {other:?}"),
    }
    shard1.validate_lru_chain();

    // Verify key2.
    let shard2 = &loaded_shards[key2.get_shard_id()];
    let slot2 = shard2.map.get(&CryptoHash::hash(&key2)).unwrap();
    match slot2.kind() {
        StateSlotKind::HotVacant {
            hot_since_version, ..
        } => {
            assert_eq!(*hot_since_version, 200);
        },
        other => panic!("Expected HotVacant for key2, got {other:?}"),
    }
    shard2.validate_lru_chain();
}
