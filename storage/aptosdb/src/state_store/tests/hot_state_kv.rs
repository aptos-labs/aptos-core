// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::AptosDB,
    pruner::PrunerManager,
    schema::{
        hot_state_value_by_key_hash::{HotStateEntry, HotStateValueByKeyHashSchema},
        stale_state_value_index_by_key_hash::StaleStateValueIndexByKeyHashSchema,
    },
    state_kv_db::{LoadedHotStateShard, StateKvDb},
    state_store::StateStore,
};
use aptos_config::config::{RocksdbConfig, StorageDirPaths};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_schemadb::batch::WriteBatch;
use aptos_storage_interface::state_store::{
    empty_hot_state_updates, HotEvictionOp, HotInsertionOp, HotStateUpdates,
};
use aptos_temppath::TempPath;
use aptos_types::{
    state_store::{
        hot_state::{HotStateValue, THotStateSlot},
        state_key::StateKey,
        state_slot::StateSlotKind,
        state_value::{StaleStateValueByKeyHashIndex, StateValue},
        NUM_STATE_SHARDS,
    },
    transaction::Version,
};
use proptest::{collection::vec, prelude::*};
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
        .get_hot_state_entry_by_version(*key.crypto_hash_ref(), Version::MAX)
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
        .get_hot_state_entry_by_version(*key.crypto_hash_ref(), Version::MAX)
        .unwrap()
        .unwrap();
    assert_eq!(latest_version, 2);
    assert_eq!(latest_entry, expected_entry);

    // Querying at V1 should return the older entry.
    let (older_version, older_entry) = db
        .get_hot_state_entry_by_version(*key.crypto_hash_ref(), 1)
        .unwrap()
        .unwrap();
    assert_eq!(older_version, 1);
    assert_eq!(older_entry, expected_entry);
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

fn assert_hot_occupied(
    shard: &LoadedHotStateShard,
    key_hash: HashValue,
    expected_hot_since_version: Version,
    expected_value_version: Version,
    expected_value: StateValue,
) {
    let slot = shard.map.get(&key_hash).unwrap();
    match slot.kind() {
        StateSlotKind::HotOccupied {
            value_version,
            value,
            hot_since_version,
            ..
        } => {
            assert_eq!(*hot_since_version, expected_hot_since_version);
            assert_eq!(*value_version, expected_value_version);
            assert_eq!(*value, expected_value);
        },
        other => panic!("Expected HotOccupied, got {other:?}"),
    }
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
    drop(slot);

    assert_hot_occupied(shard, key_hash, 10, 5, make_state_value(1));
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
fn test_load_total_value_bytes() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key_occ = make_state_key(21);
    let value = make_state_value(21);
    put_hot_state_entry(
        &db,
        &key_occ,
        100,
        Some(HotStateEntry::Occupied {
            value: value.clone(),
            value_version: 50,
        }),
    );

    let key_vac = make_state_key(22);
    put_hot_state_entry(&db, &key_vac, 200, Some(HotStateEntry::Vacant));

    let shards = db.load_hot_state_kvs(300).unwrap();
    let total_value_bytes: usize = shards.iter().map(|shard| shard.total_value_bytes).sum();
    assert_eq!(total_value_bytes, value.size());
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
    assert_hot_occupied(shard, CryptoHash::hash(&key), 20, 15, make_state_value(72));
    shard.validate_lru_chain();
}

#[test]
fn test_load_skips_future_entry_and_falls_back_to_older_version() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key = make_state_key(9);
    let key_hash = CryptoHash::hash(&key);
    put_hot_state_entry(
        &db,
        &key,
        10,
        Some(HotStateEntry::Occupied {
            value: make_state_value(91),
            value_version: 5,
        }),
    );
    put_hot_state_entry(
        &db,
        &key,
        20,
        Some(HotStateEntry::Occupied {
            value: make_state_value(92),
            value_version: 15,
        }),
    );

    let shards = db.load_hot_state_kvs(15).unwrap();
    let shard = &shards[key.get_shard_id()];
    assert_hot_occupied(shard, key_hash, 10, 5, make_state_value(91));
    shard.validate_lru_chain();
}

#[test]
fn test_load_snapshot_boundaries_across_eviction_and_reinsert() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let key = make_state_key(11);
    let key_hash = CryptoHash::hash(&key);
    let shard_id = key.get_shard_id();
    put_hot_state_entry(
        &db,
        &key,
        10,
        Some(HotStateEntry::Occupied {
            value: make_state_value(111),
            value_version: 5,
        }),
    );
    put_hot_state_entry(&db, &key, 20, None);
    put_hot_state_entry(
        &db,
        &key,
        30,
        Some(HotStateEntry::Occupied {
            value: make_state_value(112),
            value_version: 25,
        }),
    );

    let shards_at_15 = db.load_hot_state_kvs(15).unwrap();
    let shard_at_15 = &shards_at_15[shard_id];
    assert_hot_occupied(shard_at_15, key_hash, 10, 5, make_state_value(111));
    shard_at_15.validate_lru_chain();

    let shards_at_25 = db.load_hot_state_kvs(25).unwrap();
    let shard_at_25 = &shards_at_25[shard_id];
    assert!(!shard_at_25.map.contains_key(&key_hash));
    shard_at_25.validate_lru_chain();

    let shards_at_30 = db.load_hot_state_kvs(30).unwrap();
    let shard_at_30 = &shards_at_30[shard_id];
    assert_hot_occupied(shard_at_30, key_hash, 30, 25, make_state_value(112));
    shard_at_30.validate_lru_chain();
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
    assert_hot_occupied(shard, CryptoHash::hash(&key), 30, 25, make_state_value(82));
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
fn test_load_lru_chain_same_version_orders_by_key_hash() {
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);

    let mut keys_by_shard: Vec<Vec<StateKey>> = (0..NUM_STATE_SHARDS).map(|_| Vec::new()).collect();
    for seed in 0..=255u8 {
        let key = make_state_key(seed);
        keys_by_shard[key.get_shard_id()].push(key);
    }

    let (shard_id, mut keys) = keys_by_shard
        .into_iter()
        .enumerate()
        .find(|(_, keys)| keys.len() >= 3)
        .expect("at least one shard should have three keys");
    keys.truncate(3);

    for key in &keys {
        put_hot_state_entry(
            &db,
            key,
            100,
            Some(HotStateEntry::Occupied {
                value: make_state_value(100),
                value_version: 50,
            }),
        );
    }

    let shards = db.load_hot_state_kvs(100).unwrap();
    let shard = &shards[shard_id];
    shard.validate_lru_chain();

    let actual: Vec<_> = collect_lru_order(shard)
        .into_iter()
        .map(|(key_hash, hot_since_version)| {
            assert_eq!(hot_since_version, 100);
            key_hash
        })
        .collect();
    let mut expected: Vec<_> = keys.iter().map(CryptoHash::hash).collect();
    expected.sort();
    expected.reverse();

    assert_eq!(actual, expected);
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

    let mut shards = empty_hot_state_updates();

    // key1: occupied at hot_since_version=100, value_version=50
    shards[key1.get_shard_id()]
        .insertions
        .insert(*key1.crypto_hash_ref(), HotInsertionOp {
            state_key: key1.clone(),
            value: HotStateValue::new(Some(val1.clone()), 100),
            value_version: Some(50),
            superseded_version: None,
        });

    // key2: vacant at hot_since_version=200
    shards[key2.get_shard_id()]
        .insertions
        .insert(*key2.crypto_hash_ref(), HotInsertionOp {
            state_key: key2.clone(),
            value: HotStateValue::new(None, 200),
            value_version: None,
            superseded_version: None,
        });

    let hot_state_updates = HotStateUpdates {
        for_last_checkpoint: Some(shards),
        for_latest: None,
    };

    let mut sharded_batches = hot_state_kv_db.new_sharded_native_batches();
    StateStore::put_hot_state_updates(&hot_state_updates, &mut sharded_batches).unwrap();
    hot_state_kv_db.commit(999, None, sharded_batches).unwrap();

    // Load back.
    let loaded_shards = hot_state_kv_db.load_hot_state_kvs(999).unwrap();

    // Verify key1.
    let shard1 = &loaded_shards[key1.get_shard_id()];
    assert_hot_occupied(shard1, CryptoHash::hash(&key1), 100, 50, val1);
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

// ---------------------------------------------------------------------------
// Randomized end-to-end check against an oracle
// ---------------------------------------------------------------------------

fn make_state_key_u32(id: u32) -> StateKey {
    StateKey::raw(&id.to_be_bytes())
}

/// Writes many hot state entries with one native batch per shard, rather than one
/// commit per entry, so a few-hundred-key proptest case stays cheap.
fn put_hot_state_entries_bulk(
    db: &StateKvDb,
    writes: Vec<(StateKey, Version, Option<HotStateEntry>)>,
) {
    let mut by_shard: Vec<Vec<(HashValue, Version, Option<HotStateEntry>)>> =
        (0..NUM_STATE_SHARDS).map(|_| Vec::new()).collect();
    for (key, version, entry) in writes {
        by_shard[key.get_shard_id()].push((key.hash(), version, entry));
    }
    for (shard_id, entries) in by_shard.into_iter().enumerate() {
        if entries.is_empty() {
            continue;
        }
        let mut batch = db.db_shard(shard_id).new_native_batch();
        for (key_hash, version, entry) in entries {
            batch
                .put::<HotStateValueByKeyHashSchema>(&(key_hash, version), &entry)
                .unwrap();
        }
        db.db_shard(shard_id).write_schemas(batch).unwrap();
    }
}

/// One generated write, resolved against the `hot_since_version` it gets assigned.
#[derive(Clone, Debug)]
enum GenEntry {
    Occupied {
        value_seed: u8,
        value_version_delta: Version,
    },
    Vacant,
    Evicted,
}

impl GenEntry {
    fn to_entry(&self, hot_since_version: Version) -> Option<HotStateEntry> {
        match self {
            GenEntry::Occupied {
                value_seed,
                value_version_delta,
            } => Some(HotStateEntry::Occupied {
                value: make_state_value(*value_seed),
                value_version: hot_since_version.saturating_sub(*value_version_delta),
            }),
            GenEntry::Vacant => Some(HotStateEntry::Vacant),
            GenEntry::Evicted => None,
        }
    }
}

fn arb_gen_entry() -> impl Strategy<Value = GenEntry> {
    prop_oneof![
        3 => (any::<u8>(), 0u64..8).prop_map(|(value_seed, value_version_delta)| {
            GenEntry::Occupied {
                value_seed,
                value_version_delta,
            }
        }),
        1 => Just(GenEntry::Vacant),
        1 => Just(GenEntry::Evicted),
    ]
}

/// A few hundred writes over up to 400 keys, paired with a snapshot version in
/// `[0, num_writes]` so loading exercises both included and skipped (future) entries.
fn arb_load_input() -> impl Strategy<Value = (Vec<(u32, GenEntry)>, Version)> {
    vec((0u32..400, arb_gen_entry()), 300..800).prop_flat_map(|ops| {
        let num_writes = ops.len() as Version;
        (Just(ops), 0..=num_writes)
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// Build a hot state KV DB from many versioned writes, then check that
    /// `load_hot_state_kvs` at a random snapshot reproduces, for every key, the most
    /// recent non-evicted entry at or before that snapshot — with a well-formed LRU
    /// chain and matching `total_value_bytes`.
    #[test]
    fn test_load_hot_state_kvs_matches_oracle(
        (ops, snapshot_version) in arb_load_input(),
    ) {
        let tmp = TempPath::new();
        let db = create_hot_state_kv_db(&tmp);

        // Each op gets a unique, strictly increasing hot_since_version (its index).
        let mut writes = Vec::with_capacity(ops.len());
        // Oracle: per key, the latest (key, hot_since, entry) written at or before the snapshot.
        let mut oracle: HashMap<HashValue, (StateKey, Version, Option<HotStateEntry>)> =
            HashMap::new();
        for (idx, (key_id, gen_entry)) in ops.into_iter().enumerate() {
            let version = idx as Version;
            let key = make_state_key_u32(key_id);
            let entry = gen_entry.to_entry(version);
            if version <= snapshot_version {
                oracle.insert(key.hash(), (key.clone(), version, entry.clone()));
            }
            writes.push((key, version, entry));
        }
        put_hot_state_entries_bulk(&db, writes);

        let shards = db.load_hot_state_kvs(snapshot_version).unwrap();

        // Verify each expected slot and collect the expected LRU members per shard.
        let mut expected_chain: Vec<Vec<(HashValue, Version)>> =
            vec![Vec::new(); NUM_STATE_SHARDS];
        let mut expected_total_value_bytes = 0usize;
        for (key, hot_since, entry) in oracle.values() {
            let shard_id = key.get_shard_id();
            let key_hash = key.hash();
            let expected_entry = match entry {
                None => {
                    // Evicted: must be absent after load.
                    prop_assert!(
                        !shards[shard_id].map.contains_key(&key_hash),
                        "evicted key {key_hash} (shard {shard_id}) should be absent"
                    );
                    continue;
                },
                Some(e) => e,
            };
            expected_chain[shard_id].push((key_hash, *hot_since));

            let slot = shards[shard_id].map.get(&key_hash);
            prop_assert!(
                slot.is_some(),
                "key {key_hash} (shard {shard_id}) missing from loaded shard"
            );
            let slot = slot.unwrap();
            prop_assert_eq!(slot.expect_hot_since_version(), *hot_since);
            match (slot.kind(), expected_entry) {
                (
                    StateSlotKind::HotOccupied {
                        value,
                        value_version,
                        ..
                    },
                    HotStateEntry::Occupied {
                        value: exp_value,
                        value_version: exp_vv,
                    },
                ) => {
                    prop_assert_eq!(value, exp_value);
                    prop_assert_eq!(value_version, exp_vv);
                    expected_total_value_bytes += exp_value.size();
                },
                (StateSlotKind::HotVacant { .. }, HotStateEntry::Vacant) => {},
                (actual, _) => prop_assert!(
                    false,
                    "kind mismatch for {key_hash}: {actual:?} vs {expected_entry:?}"
                ),
            }
        }

        // Every shard's chain is well-formed and ordered by (hot_since_version, key_hash)
        // descending head→tail; num_items and total_value_bytes match the oracle exactly.
        for (shard_id, shard) in shards.iter().enumerate() {
            shard.validate_lru_chain();
            prop_assert_eq!(shard.num_items, expected_chain[shard_id].len());

            let mut expected = expected_chain[shard_id].clone();
            expected.sort_by_key(|(key_hash, version)| (*version, *key_hash));
            expected.reverse();
            prop_assert_eq!(collect_lru_order(shard), expected);
        }

        let actual_total_value_bytes: usize = shards.iter().map(|s| s.total_value_bytes).sum();
        prop_assert_eq!(actual_total_value_bytes, expected_total_value_bytes);
    }
}

// ---------------------------------------------------------------------------
// Stale index and pruning tests
// ---------------------------------------------------------------------------

/// Collect all stale index entries from the given shard DB.
fn collect_stale_indices(db: &StateKvDb, shard_id: usize) -> Vec<StaleStateValueByKeyHashIndex> {
    let mut iter = db
        .db_shard(shard_id)
        .iter::<StaleStateValueIndexByKeyHashSchema>()
        .unwrap();
    iter.seek(&0u64).unwrap();
    iter.map(|item| item.unwrap().0).collect()
}

#[test]
fn test_stale_index_direct_write_read() {
    // Minimal test: verify the stale index CF works on a hot state KV DB.
    let tmp = TempPath::new();
    let db = create_hot_state_kv_db(&tmp);
    let key = make_state_key(1);
    let shard_id = key.get_shard_id();

    let idx = StaleStateValueByKeyHashIndex {
        stale_since_version: 100,
        version: 50,
        state_key_hash: CryptoHash::hash(&key),
    };

    let mut batch = db.db_shard(shard_id).new_native_batch();
    batch
        .put::<StaleStateValueIndexByKeyHashSchema>(&idx, &())
        .unwrap();
    db.db_shard(shard_id).write_schemas(batch).unwrap();

    let entries = collect_stale_indices(&db, shard_id);
    assert_eq!(
        entries.len(),
        1,
        "should find 1 stale entry, found {}",
        entries.len()
    );
    assert_eq!(entries[0].stale_since_version, 100);
    assert_eq!(entries[0].version, 50);
}

#[test]
fn test_put_hot_state_updates_values_and_stale_indices() {
    let tmp = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp);
    let hot_state_kv_db = aptos_db.hot_state_kv_db.as_ref().unwrap();

    let key1 = make_state_key(10);
    let val1 = make_state_value(10);
    let key2 = make_state_key(20);
    let key3 = make_state_key(30);

    let mut shards = empty_hot_state_updates();

    // key1: first write (no superseded version) at hot_since_version=100
    shards[key1.get_shard_id()]
        .insertions
        .insert(*key1.crypto_hash_ref(), HotInsertionOp {
            state_key: key1.clone(),
            value: HotStateValue::new(Some(val1.clone()), 100),
            value_version: Some(50),
            superseded_version: None,
        });

    // key2: insertion superseding an old entry at hot_since_version=80
    shards[key2.get_shard_id()]
        .insertions
        .insert(*key2.crypto_hash_ref(), HotInsertionOp {
            state_key: key2.clone(),
            value: HotStateValue::new(None, 200),
            value_version: None,
            superseded_version: Some(80),
        });

    // key3: eviction only (key was hot from before at hot_since=150, now evicted at 300).
    shards[key3.get_shard_id()]
        .evictions
        .insert(*key3.crypto_hash_ref(), HotEvictionOp {
            eviction_version: 300,
            superseded_version: Some(150),
        });

    let hot_state_updates = HotStateUpdates {
        for_last_checkpoint: Some(shards),
        for_latest: None,
    };

    let mut sharded_batches = hot_state_kv_db.new_sharded_native_batches();
    StateStore::put_hot_state_updates(&hot_state_updates, &mut sharded_batches).unwrap();
    hot_state_kv_db.commit(999, None, sharded_batches).unwrap();

    // -- Verify value entries --

    // key1: occupied
    assert_eq!(
        get_hot_state_entry(hot_state_kv_db, &key1, 100).unwrap(),
        HotStateEntry::Occupied {
            value: val1,
            value_version: 50,
        }
    );
    // key2: vacant
    assert_eq!(
        get_hot_state_entry(hot_state_kv_db, &key2, 200).unwrap(),
        HotStateEntry::Vacant,
    );
    // key3: eviction (None)
    assert!(
        get_hot_state_entry(hot_state_kv_db, &key3, 300).is_none(),
        "Eviction should be None"
    );

    // -- Verify stale index entries --

    // Check all shards for any stale entries
    let total_stale: usize = (0..NUM_STATE_SHARDS)
        .map(|s| collect_stale_indices(hot_state_kv_db, s).len())
        .sum();
    assert!(
        total_stale > 0,
        "expected some stale entries across all shards, found 0. key1 shard={}, key2 shard={}, key3 shard={}",
        key1.get_shard_id(), key2.get_shard_id(), key3.get_shard_id()
    );

    // key1: should have one stale index entry with NO_PREV_VERSION (first write)
    let stale1 = collect_stale_indices(hot_state_kv_db, key1.get_shard_id());
    let key1_entries: Vec<_> = stale1
        .iter()
        .filter(|e| e.state_key_hash == CryptoHash::hash(&key1))
        .collect();
    assert_eq!(key1_entries.len(), 1);
    assert_eq!(key1_entries[0].stale_since_version, 100);
    assert!(key1_entries[0].is_first_write());

    // key2: should have one stale index entry with old version 80
    let stale2 = collect_stale_indices(hot_state_kv_db, key2.get_shard_id());
    let key2_entries: Vec<_> = stale2
        .iter()
        .filter(|e| e.state_key_hash == CryptoHash::hash(&key2))
        .collect();
    assert_eq!(key2_entries.len(), 1);
    assert_eq!(key2_entries[0].stale_since_version, 200);
    assert_eq!(key2_entries[0].version, 80);

    // key3: should have two stale index entries from the eviction:
    //   1. stale_since=300, version=150 (old entry superseded by eviction)
    //   2. stale_since=300, version=300 (tombstone self-ref)
    let stale3 = collect_stale_indices(hot_state_kv_db, key3.get_shard_id());
    let mut key3_entries: Vec<_> = stale3
        .iter()
        .filter(|e| e.state_key_hash == CryptoHash::hash(&key3))
        .cloned()
        .collect();
    key3_entries.sort_by_key(|e| (e.stale_since_version, e.version));
    assert_eq!(
        key3_entries.len(),
        2,
        "key3 stale entries: {key3_entries:#?}"
    );
    // Eviction supersedes old entry at 150
    assert_eq!(key3_entries[0].stale_since_version, 300);
    assert_eq!(key3_entries[0].version, 150);
    // Self-referential tombstone
    assert_eq!(key3_entries[1].stale_since_version, 300);
    assert_eq!(key3_entries[1].version, 300);
}

#[test]
fn test_hot_state_kv_pruner_deletes_old_entries() {
    use aptos_config::config::{
        HotStateConfig, PrunerConfig, RocksdbConfigs, StorageDirPaths,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    };

    let tmp = TempPath::new();
    // Open with pruner enabled (prune_window=0 so everything is immediately prunable).
    let mut pruner_config = PrunerConfig::default();
    pruner_config.ledger_pruner_config.enable = true;
    pruner_config.ledger_pruner_config.prune_window = 0;
    pruner_config.ledger_pruner_config.batch_size = 1;
    let aptos_db = AptosDB::open(
        StorageDirPaths::from_path(tmp.path()),
        false,
        pruner_config,
        RocksdbConfigs::default(),
        500_000,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
        None,
        HotStateConfig::default(),
    )
    .unwrap();
    let hot_state_kv_db = aptos_db.hot_state_kv_db.as_ref().unwrap();

    let key1 = make_state_key(42);
    let val_old = make_state_value(1);
    let val_new = make_state_value(2);

    // First batch: write old entry at hot_since=100
    let mut shards = empty_hot_state_updates();
    shards[key1.get_shard_id()]
        .insertions
        .insert(*key1.crypto_hash_ref(), HotInsertionOp {
            state_key: key1.clone(),
            value: HotStateValue::new(Some(val_old.clone()), 100),
            value_version: Some(100),
            superseded_version: None,
        });
    let updates1 = HotStateUpdates {
        for_last_checkpoint: Some(shards),
        for_latest: None,
    };
    let mut batches = hot_state_kv_db.new_sharded_native_batches();
    StateStore::put_hot_state_updates(&updates1, &mut batches).unwrap();
    hot_state_kv_db.commit(100, None, batches).unwrap();

    // Second batch: write new entry at hot_since=200 superseding 100
    let mut shards2 = empty_hot_state_updates();
    shards2[key1.get_shard_id()]
        .insertions
        .insert(*key1.crypto_hash_ref(), HotInsertionOp {
            state_key: key1.clone(),
            value: HotStateValue::new(Some(val_new.clone()), 200),
            value_version: Some(200),
            superseded_version: Some(100),
        });
    let updates2 = HotStateUpdates {
        for_last_checkpoint: Some(shards2),
        for_latest: None,
    };
    let mut batches2 = hot_state_kv_db.new_sharded_native_batches();
    StateStore::put_hot_state_updates(&updates2, &mut batches2).unwrap();
    hot_state_kv_db.commit(200, None, batches2).unwrap();

    // Both entries exist before pruning
    assert!(get_hot_state_entry(hot_state_kv_db, &key1, 100).is_some());
    assert!(get_hot_state_entry(hot_state_kv_db, &key1, 200).is_some());

    // Trigger pruning
    let pruner = aptos_db
        .state_store
        .state_pruner
        .hot_state_kv_pruner
        .as_ref()
        .expect("hot state kv pruner should exist");
    pruner
        .wake_and_wait_pruner(200)
        .expect("pruner should complete");

    // Old entry at version 100 should be pruned
    let old_entry = hot_state_kv_db
        .db_shard(key1.get_shard_id())
        .get::<HotStateValueByKeyHashSchema>(&(CryptoHash::hash(&key1), 100))
        .unwrap();
    assert!(
        old_entry.is_none(),
        "old entry should have been pruned, got: {old_entry:?}"
    );
    // New entry at version 200 should remain
    assert!(get_hot_state_entry(hot_state_kv_db, &key1, 200).is_some());

    // Stale index entries should be cleaned up
    let stale = collect_stale_indices(hot_state_kv_db, key1.get_shard_id());
    let key1_stale: Vec<_> = stale
        .iter()
        .filter(|e| e.state_key_hash == CryptoHash::hash(&key1))
        .filter(|e| e.stale_since_version <= 200)
        .collect();
    assert!(
        key1_stale.is_empty(),
        "stale entries should be cleaned up, found: {key1_stale:?}"
    );
}
