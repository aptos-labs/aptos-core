// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for stale state value index entries, verifying that first-time key creations produce
//! sentinel stale index entries so truncation can discover and clean them up.

use super::*;
use crate::schema::{STALE_STATE_VALUE_INDEX_BY_KEY_HASH_CF_NAME, STATE_VALUE_BY_KEY_HASH_CF_NAME};
use aptos_schemadb::{Options, DB};
use aptos_temppath::TempPath;
use aptos_types::write_set::BaseStateOp;

fn open_test_kv_db(path: &TempPath) -> DB {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    DB::open(
        path.path(),
        "test_kv_shard",
        vec![
            "default",
            STALE_STATE_VALUE_INDEX_BY_KEY_HASH_CF_NAME,
            STATE_VALUE_BY_KEY_HASH_CF_NAME,
        ],
        opts,
    )
    .unwrap()
}

/// Collect all stale index entries from the DB, sorted by (stale_since_version, version).
fn collect_stale_indices(db: &DB) -> Vec<StaleStateValueByKeyHashIndex> {
    let mut iter = db.iter::<StaleStateValueIndexByKeyHashSchema>().unwrap();
    iter.seek_to_first();
    iter.map(|item| item.unwrap().0).collect()
}

/// Collect all (state_key_hash, version) pairs from StateValueByKeyHashSchema.
fn collect_state_values(db: &DB) -> Vec<(HashValue, Version)> {
    let mut iter = db.iter::<StateValueByKeyHashSchema>().unwrap();
    iter.seek_to_first();
    iter.map(|item| item.unwrap().0).collect()
}

/// Replicate the truncation logic from `delete_state_value_and_index` to verify it works
/// correctly with sentinel stale index entries.
fn truncate_state_values(db: &DB, start_version: Version) {
    let mut batch = SchemaBatch::new();
    let mut iter = db.iter::<StaleStateValueIndexByKeyHashSchema>().unwrap();
    iter.seek(&start_version).unwrap();
    for item in iter {
        let (index, _) = item.unwrap();
        batch
            .delete::<StaleStateValueIndexByKeyHashSchema>(&index)
            .unwrap();
        batch
            .delete::<StateValueByKeyHashSchema>(&(index.state_key_hash, index.stale_since_version))
            .unwrap();
    }
    db.write_schemas(batch).unwrap();
}

/// Verify that `put_stale_state_value_index_for_shard` writes sentinel stale index entries for
/// first-time key creations and normal entries for updates. Then verify that truncation using
/// these entries correctly cleans up all state values beyond the truncation point.
#[test]
fn test_stale_index_for_first_write_and_truncation() {
    let tmp = TempPath::new();
    let db = open_test_kv_db(&tmp);

    let key_new = StateKey::raw(b"brand_new_key");
    let key_existing = StateKey::raw(b"existing_key");
    let val = StateValue::from(b"some_value".to_vec());

    // Pre-populate cache: key_new never existed (ColdVacant), key_existing was at version 5.
    let cache: StateCacheShard = dashmap::DashMap::new();
    cache.insert(key_new.clone(), StateSlot::ColdVacant);
    cache.insert(key_existing.clone(), StateSlot::ColdOccupied {
        value_version: 5,
        value: val.clone(),
    });

    // Both keys are written at version 10.
    let op_new = BaseStateOp::Creation(val.clone());
    let op_existing = BaseStateOp::Modification(val.clone());
    let updates: Vec<(&StateKey, StateUpdateRef)> = vec![
        (&key_new, StateUpdateRef {
            version: 10,
            state_op: &op_new,
        }),
        (&key_existing, StateUpdateRef {
            version: 10,
            state_op: &op_existing,
        }),
    ];

    // --- Phase 1: verify stale index entries ---
    let mut batch = db.new_native_batch();
    StateStore::put_stale_state_value_index_for_shard(
        0,  // shard_id
        10, // first_version
        1,  // num_versions (just version 10)
        &cache, &updates, &mut batch, false, // ignore_state_cache_miss
    );
    db.write_schemas(batch).unwrap();

    let entries = collect_stale_indices(&db);
    assert_eq!(entries.len(), 2, "expected stale entries for both keys");

    let new_hash = CryptoHash::hash(&key_new);
    let existing_hash = CryptoHash::hash(&key_existing);

    let new_entry = entries
        .iter()
        .find(|e| e.state_key_hash == new_hash)
        .expect("missing stale index for first-time creation");
    assert_eq!(new_entry.stale_since_version, 10);
    assert_eq!(
        new_entry.version, VERSION_PLACEHOLDER_FOR_FIRST_WRITE,
        "first-time creation should use sentinel version"
    );

    let existing_entry = entries
        .iter()
        .find(|e| e.state_key_hash == existing_hash)
        .expect("missing stale index for update");
    assert_eq!(existing_entry.stale_since_version, 10);
    assert_eq!(
        existing_entry.version, 5,
        "update should reference the old version"
    );

    // --- Phase 2: verify truncation cleans up all entries ---
    // Write state value entries that the truncation should delete.
    let mut value_batch = SchemaBatch::new();
    // key_existing had a value at v5 (should survive truncation to v5).
    value_batch
        .put::<StateValueByKeyHashSchema>(&(existing_hash, 5), &Some(val.clone()))
        .unwrap();
    // Both keys have values at v10 (should be removed by truncation to v5).
    value_batch
        .put::<StateValueByKeyHashSchema>(&(new_hash, 10), &Some(val.clone()))
        .unwrap();
    value_batch
        .put::<StateValueByKeyHashSchema>(&(existing_hash, 10), &Some(val.clone()))
        .unwrap();
    db.write_schemas(value_batch).unwrap();

    // 3 state value entries before truncation.
    assert_eq!(collect_state_values(&db).len(), 3);

    // Truncate: delete everything with stale_since_version >= 6 (i.e., truncate to version 5).
    truncate_state_values(&db, 6);

    // After truncation: only key_existing at v5 should remain.
    let remaining = collect_state_values(&db);
    assert_eq!(
        remaining.len(),
        1,
        "only the pre-existing value should survive"
    );
    assert_eq!(remaining[0], (existing_hash, 5));

    // No stale index entries should remain (both had stale_since_version = 10 >= 6).
    assert!(collect_stale_indices(&db).is_empty());
}

/// Verify that a key created, then updated in a later version, produces the right stale index
/// entries for both versions — and truncation between the two versions cleans up correctly.
#[test]
fn test_stale_index_create_then_update_truncation() {
    let tmp = TempPath::new();
    let db = open_test_kv_db(&tmp);

    let key = StateKey::raw(b"the_key");
    let val_v1 = StateValue::from(b"v1".to_vec());
    let val_v2 = StateValue::from(b"v2".to_vec());
    let key_hash = CryptoHash::hash(&key);

    // Version 10: first-time creation of key.
    {
        let cache: StateCacheShard = dashmap::DashMap::new();
        cache.insert(key.clone(), StateSlot::ColdVacant);

        let op = BaseStateOp::Creation(val_v1.clone());
        let updates: Vec<(&StateKey, StateUpdateRef)> = vec![(&key, StateUpdateRef {
            version: 10,
            state_op: &op,
        })];

        let mut batch = db.new_native_batch();
        StateStore::put_stale_state_value_index_for_shard(
            0, 10, 1, &cache, &updates, &mut batch, false,
        );
        db.write_schemas(batch).unwrap();
    }

    // Version 20: update the key.
    {
        // Cache reflects that the key was written at v10 (by the insert() in the previous call,
        // the cache now holds the v10 slot). We need to re-populate the cache since we're
        // simulating a fresh commit.
        let cache: StateCacheShard = dashmap::DashMap::new();
        cache.insert(key.clone(), StateSlot::ColdOccupied {
            value_version: 10,
            value: val_v1.clone(),
        });

        let op = BaseStateOp::Modification(val_v2.clone());
        let updates: Vec<(&StateKey, StateUpdateRef)> = vec![(&key, StateUpdateRef {
            version: 20,
            state_op: &op,
        })];

        let mut batch = db.new_native_batch();
        StateStore::put_stale_state_value_index_for_shard(
            0, 20, 1, &cache, &updates, &mut batch, false,
        );
        db.write_schemas(batch).unwrap();
    }

    // Verify stale index entries:
    // 1. (stale_since=10, version=MAX, key_hash) — first-time creation sentinel
    // 2. (stale_since=20, version=10, key_hash)  — v10 became stale at v20
    let entries = collect_stale_indices(&db);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].stale_since_version, 10);
    assert_eq!(entries[0].version, VERSION_PLACEHOLDER_FOR_FIRST_WRITE);
    assert_eq!(entries[1].stale_since_version, 20);
    assert_eq!(entries[1].version, 10);

    // Write state value entries.
    let mut value_batch = SchemaBatch::new();
    value_batch
        .put::<StateValueByKeyHashSchema>(&(key_hash, 10), &Some(val_v1))
        .unwrap();
    value_batch
        .put::<StateValueByKeyHashSchema>(&(key_hash, 20), &Some(val_v2))
        .unwrap();
    db.write_schemas(value_batch).unwrap();

    // Truncate to version 15: should remove v20 entry and v10 entry (first-time creation).
    // Both stale_since_version=10 and =20 are >= 16? No — only stale_since=20 is >= 16.
    // stale_since=10 is < 16, so the sentinel entry is NOT in the truncation range.
    // This means v10 survives (correct — it's before the truncation point).
    truncate_state_values(&db, 16);

    let remaining = collect_state_values(&db);
    assert_eq!(remaining.len(), 1);
    assert_eq!(
        remaining[0],
        (key_hash, 10),
        "v10 should survive truncation to v15"
    );

    // Only the sentinel stale index at stale_since=10 should remain.
    let remaining_indices = collect_stale_indices(&db);
    assert_eq!(remaining_indices.len(), 1);
    assert_eq!(remaining_indices[0].stale_since_version, 10);
    assert_eq!(
        remaining_indices[0].version,
        VERSION_PLACEHOLDER_FOR_FIRST_WRITE
    );
}
