// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for the server-side hot state snapshot reads (`get_hot_state_item_count` and
//! `get_hot_state_value_chunk_iter`), which a fast-syncing peer uses to rebuild the hot state.
//!
//! Each test seeds the hot state KV DB and the hot state Merkle tree directly with a consistent
//! set of leaves at a version, then reads them back through the `DbReader` surface.

use crate::{
    db::AptosDB,
    schema::hot_state_value_by_key_hash::{HotStateEntry, HotStateValueByKeyHashSchema},
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_schemadb::batch::WriteBatch;
use aptos_storage_interface::{DbReader, Result};
use aptos_temppath::TempPath;
use aptos_types::{
    state_store::{
        hot_state::{HotStateValue, HotStateValueChunkItem},
        state_key::StateKey,
        state_value::StateValue,
    },
    transaction::Version,
};

fn make_state_key(seed: u8) -> StateKey {
    StateKey::raw(&[seed])
}

fn make_state_value(seed: u8) -> StateValue {
    StateValue::new_legacy(vec![seed; (seed % 16) as usize + 1].into())
}

/// A hot leaf to seed into both hot state DBs. `value`/`value_version` are `Some` for occupied
/// entries and `None` for vacant ones.
struct Leaf {
    key: StateKey,
    value: Option<StateValue>,
    hot_since_version: Version,
    value_version: Option<Version>,
}

impl Leaf {
    fn occupied(seed: u8, hot_since_version: Version, value_version: Version) -> Self {
        Self {
            key: make_state_key(seed),
            value: Some(make_state_value(seed)),
            hot_since_version,
            value_version: Some(value_version),
        }
    }

    fn vacant(seed: u8, hot_since_version: Version) -> Self {
        Self {
            key: make_state_key(seed),
            value: None,
            hot_since_version,
            value_version: None,
        }
    }

    fn to_entry(&self) -> HotStateEntry {
        match (&self.value, self.value_version) {
            (Some(value), Some(value_version)) => HotStateEntry::Occupied {
                value: value.clone(),
                value_version,
            },
            (None, None) => HotStateEntry::Vacant,
            _ => panic!("value and value_version must both be set or unset"),
        }
    }
}

/// Writes `leaves` into the hot state KV DB and the hot state Merkle tree, committing the tree at
/// `version`. Mirrors what a normal snapshot commit produces: the Merkle leaf hashes the
/// `HotStateValue` (value + hot_since_version) and the KV holds the full entry.
fn seed_hot_state(db: &AptosDB, leaves: &[Leaf], version: Version) {
    let store = &db.state_store;

    // 1. Hot state KV entries, keyed by (key_hash, hot_since_version).
    for leaf in leaves {
        let shard_id = leaf.key.get_shard_id();
        let mut batch = store.hot_state_kv_db.db_shard(shard_id).new_native_batch();
        batch
            .put::<HotStateValueByKeyHashSchema>(
                &(leaf.key.hash(), leaf.hot_since_version),
                &Some(leaf.to_entry()),
            )
            .unwrap();
        store
            .hot_state_kv_db
            .db_shard(shard_id)
            .write_schemas(batch)
            .unwrap();
    }

    // 2. Hot state Merkle leaves: (key_hash, (HotStateValue hash, key)).
    let leaf_values: Vec<(HashValue, (HashValue, StateKey))> = leaves
        .iter()
        .map(|leaf| {
            let value_hash = HotStateValue::new(leaf.value.clone(), leaf.hot_since_version).hash();
            (leaf.key.hash(), (value_hash, leaf.key.clone()))
        })
        .collect();
    let value_set = leaf_values
        .iter()
        .map(|(key_hash, leaf)| (*key_hash, Some(leaf)))
        .collect();
    let (top_levels_batch, sharded_batches, _root) = store
        .hot_state_merkle_db
        .merklize_value_set(value_set, version, None, None)
        .unwrap();
    store
        .hot_state_merkle_db
        .commit(version, top_levels_batch, sharded_batches)
        .unwrap();
}

/// `leaves` sorted into the order the Merkle iterator yields them (ascending key hash).
fn in_key_hash_order(leaves: &[Leaf]) -> Vec<&Leaf> {
    let mut ordered: Vec<_> = leaves.iter().collect();
    ordered.sort_by_key(|leaf| leaf.key.hash());
    ordered
}

fn assert_item_matches(item: &HotStateValueChunkItem, leaf: &Leaf) {
    assert_eq!(item.key, leaf.key);
    assert_eq!(item.value.hot_since_version(), leaf.hot_since_version);
    assert_eq!(item.value.value_opt(), leaf.value.as_ref());
    assert_eq!(item.value_version, leaf.value_version);
}

fn read_chunk(
    db: &AptosDB,
    version: Version,
    first_index: usize,
    chunk_size: usize,
) -> Vec<HotStateValueChunkItem> {
    db.get_hot_state_value_chunk_iter(version, first_index, chunk_size)
        .unwrap()
        .collect::<Result<Vec<_>>>()
        .unwrap()
}

#[test]
fn test_item_count_and_full_chunk() {
    let tmp = TempPath::new();
    let db = AptosDB::new_for_test(&tmp);
    let version = 100;

    let leaves = vec![
        Leaf::occupied(1, 10, 5),
        Leaf::occupied(2, 20, 15),
        Leaf::vacant(3, 30),
        Leaf::occupied(4, 40, 35),
    ];
    seed_hot_state(&db, &leaves, version);

    assert_eq!(db.get_hot_state_item_count(version).unwrap(), leaves.len());

    let items = read_chunk(&db, version, 0, leaves.len());
    assert_eq!(items.len(), leaves.len());
    for (item, leaf) in items.iter().zip(in_key_hash_order(&leaves)) {
        assert_item_matches(item, leaf);
    }
}

#[test]
fn test_chunk_boundaries() {
    let tmp = TempPath::new();
    let db = AptosDB::new_for_test(&tmp);
    let version = 100;

    let leaves: Vec<_> = (0..10u8).map(|i| Leaf::occupied(i, 10, 5)).collect();
    seed_hot_state(&db, &leaves, version);
    let ordered = in_key_hash_order(&leaves);

    // A mid-range chunk returns exactly `chunk_size` leaves starting at `first_index`.
    let chunk = read_chunk(&db, version, 3, 4);
    assert_eq!(chunk.len(), 4);
    for (item, leaf) in chunk.iter().zip(&ordered[3..7]) {
        assert_item_matches(item, leaf);
    }

    // A chunk size beyond the end is clamped to the remaining leaves.
    let tail = read_chunk(&db, version, 7, 100);
    assert_eq!(tail.len(), 3);
    for (item, leaf) in tail.iter().zip(&ordered[7..]) {
        assert_item_matches(item, leaf);
    }

    // Starting at the end yields nothing.
    assert!(read_chunk(&db, version, leaves.len(), 10).is_empty());
}

#[test]
fn test_vacant_only() {
    let tmp = TempPath::new();
    let db = AptosDB::new_for_test(&tmp);
    let version = 50;

    let leaves = vec![Leaf::vacant(7, 42)];
    seed_hot_state(&db, &leaves, version);

    assert_eq!(db.get_hot_state_item_count(version).unwrap(), 1);
    let items = read_chunk(&db, version, 0, 1);
    assert_eq!(items.len(), 1);
    assert_item_matches(&items[0], &leaves[0]);
}
