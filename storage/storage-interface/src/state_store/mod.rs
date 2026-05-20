// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod hot_state;
pub mod state;
pub mod state_delta;
pub mod state_summary;
pub mod state_update_refs;
pub mod state_view;
pub mod state_with_summary;
pub mod versioned_state_value;

use anyhow::{bail, Result};
use aptos_crypto::HashValue;
use aptos_types::{
    state_store::{hot_state::HotStateValue, state_key::StateKey, NUM_STATE_SHARDS},
    transaction::Version,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct HotInsertionOp {
    pub state_key: StateKey,
    pub value: HotStateValue,
    /// `Some(version)` for occupied entries and `None` for vacant.
    pub value_version: Option<Version>,
    /// The `hot_since_version` of the DB entry being superseded.
    /// `None` means this is a first write (creation or promotion).
    pub superseded_version: Option<Version>,
}

#[derive(Clone, Debug)]
pub struct HotEvictionOp {
    pub eviction_version: Version,
    /// The `hot_since_version` of the DB entry being superseded. `None` if the key was never
    /// persisted to hot DB (e.g. promoted and evicted in the same batch, unlikely though).
    pub superseded_version: Option<Version>,
}

#[derive(Clone, Debug, Default)]
pub struct HotStateShardUpdates {
    pub insertions: HashMap<HashValue, HotInsertionOp>,
    // TODO(HotState): per-block eviction tracking will be needed for cold-write elimination.
    pub evictions: HashMap<HashValue, HotEvictionOp>,
}

impl HotStateShardUpdates {
    /// Inserts `op` at `key_hash`, preserving the original DB-level `superseded_version`
    /// across insert/evict/reinsert chains: if an earlier eviction or insertion for this
    /// key already exists, its `superseded_version` is inherited so the pruner still
    /// targets the original DB entry.
    pub(crate) fn insert(&mut self, key_hash: HashValue, mut op: HotInsertionOp) {
        if let Some(evicted) = self.evictions.remove(&key_hash) {
            op.superseded_version = evicted.superseded_version;
        } else if let Some(prev) = self.insertions.get(&key_hash) {
            op.superseded_version = prev.superseded_version;
        }
        self.insertions.insert(key_hash, op);
    }

    /// Records an eviction at `key_hash`. If an earlier insertion for this key exists,
    /// it is removed and its `superseded_version` is carried onto `evict`. Returns an error
    /// if the key was already evicted — the caller is expected to never evict the same key
    /// twice within one `HotStateShardUpdates`.
    pub(crate) fn evict(&mut self, key_hash: HashValue, mut evict: HotEvictionOp) -> Result<()> {
        if self.evictions.contains_key(&key_hash) {
            bail!("Key {key_hash} cannot be evicted twice.");
        }
        if let Some(prev) = self.insertions.remove(&key_hash) {
            evict.superseded_version = prev.superseded_version;
        }
        self.evictions.insert(key_hash, evict);
        Ok(())
    }

    /// Merges `other` into `self` as the logically later batch. On key collisions, the
    /// earlier `superseded_version` wins so the pruner still targets the original DB row.
    pub fn merge(&mut self, other: HotStateShardUpdates) {
        for (key_hash, op) in other.insertions {
            self.insert(key_hash, op);
        }
        for (key_hash, mut evict) in other.evictions {
            // Like `self.evict`, but tolerate an existing eviction — re-evicting across
            // batches is legitimate (key was re-hotted between them).
            if let Some(existing) = self.evictions.remove(&key_hash) {
                evict.superseded_version = existing.superseded_version;
            } else if let Some(prev) = self.insertions.remove(&key_hash) {
                evict.superseded_version = prev.superseded_version;
            }
            self.evictions.insert(key_hash, evict);
        }
    }
}

/// Creates an empty `[HotStateShardUpdates; NUM_STATE_SHARDS]`.
pub fn empty_hot_state_updates() -> [HotStateShardUpdates; NUM_STATE_SHARDS] {
    std::array::from_fn(|_| HotStateShardUpdates::default())
}

#[derive(Clone, Debug)]
pub struct HotStateUpdates {
    pub for_last_checkpoint: Option<[HotStateShardUpdates; NUM_STATE_SHARDS]>,
    pub for_latest: Option<[HotStateShardUpdates; NUM_STATE_SHARDS]>,
}

impl HotStateUpdates {
    pub fn new_empty() -> Self {
        Self {
            for_last_checkpoint: None,
            for_latest: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(n: u8) -> HashValue {
        HashValue::new([n; HashValue::LENGTH])
    }

    fn insertion(superseded: Option<Version>, value_version: Option<Version>) -> HotInsertionOp {
        HotInsertionOp {
            state_key: StateKey::raw(b"test"),
            value: HotStateValue::new(None, value_version.unwrap_or(0)),
            value_version,
            superseded_version: superseded,
        }
    }

    fn eviction(eviction_version: Version, superseded: Option<Version>) -> HotEvictionOp {
        HotEvictionOp {
            eviction_version,
            superseded_version: superseded,
        }
    }

    #[test]
    fn insert_into_empty() {
        let mut updates = HotStateShardUpdates::default();
        updates.insert(hash(1), insertion(Some(5), Some(10)));
        assert!(updates.evictions.is_empty());
        let op = updates.insertions.get(&hash(1)).unwrap();
        assert_eq!(op.superseded_version, Some(5));
        assert_eq!(op.value_version, Some(10));
    }

    #[test]
    fn reinsert_preserves_original_superseded() {
        let mut updates = HotStateShardUpdates::default();
        updates.insert(hash(1), insertion(Some(5), Some(10)));
        updates.insert(hash(1), insertion(Some(999), Some(20)));
        let op = updates.insertions.get(&hash(1)).unwrap();
        // Superseded carries over from the first insertion; value is taken from the
        // latest insertion.
        assert_eq!(op.superseded_version, Some(5));
        assert_eq!(op.value_version, Some(20));
    }

    #[test]
    fn insert_after_eviction_inherits_superseded() {
        let mut updates = HotStateShardUpdates::default();
        updates.evict(hash(1), eviction(100, Some(5))).unwrap();
        updates.insert(hash(1), insertion(Some(999), Some(20)));
        assert!(updates.evictions.is_empty());
        let op = updates.insertions.get(&hash(1)).unwrap();
        assert_eq!(op.superseded_version, Some(5));
        assert_eq!(op.value_version, Some(20));
    }

    #[test]
    fn evict_into_empty() {
        let mut updates = HotStateShardUpdates::default();
        updates.evict(hash(1), eviction(100, Some(5))).unwrap();
        assert!(updates.insertions.is_empty());
        let ev = updates.evictions.get(&hash(1)).unwrap();
        assert_eq!(ev.eviction_version, 100);
        assert_eq!(ev.superseded_version, Some(5));
    }

    #[test]
    fn evict_after_insertion_inherits_superseded() {
        let mut updates = HotStateShardUpdates::default();
        updates.insert(hash(1), insertion(Some(5), Some(10)));
        updates.evict(hash(1), eviction(100, Some(999))).unwrap();
        assert!(updates.insertions.is_empty());
        let ev = updates.evictions.get(&hash(1)).unwrap();
        assert_eq!(ev.eviction_version, 100);
        assert_eq!(ev.superseded_version, Some(5));
    }

    #[test]
    fn evict_twice_errors() {
        let mut updates = HotStateShardUpdates::default();
        updates.evict(hash(1), eviction(100, Some(5))).unwrap();
        assert!(updates.evict(hash(1), eviction(200, Some(6))).is_err());
    }

    #[test]
    fn merge_preserves_earliest_superseded_across_double_evict() {
        // `self` already evicted k with the original DB entry at version 5.
        let mut pending = HotStateShardUpdates::default();
        pending.evict(hash(1), eviction(100, Some(5))).unwrap();

        // `other` re-hot's k via an insert (with a bogus superseded, since LRU didn't
        // know about the DB entry) and evicts it again at the next checkpoint. Within
        // one `HotStateShardUpdates`, insert+evict collapses to just an eviction.
        let mut chunk = HotStateShardUpdates::default();
        chunk.insert(hash(1), insertion(None, Some(150)));
        chunk.evict(hash(1), eviction(200, Some(999))).unwrap();
        assert!(chunk.insertions.is_empty());

        pending.merge(chunk);

        // A single eviction survives at the later `eviction_version`, but the
        // original DB-level `superseded_version` (Some(5)) is preserved so the pruner
        // still targets the right entry.
        assert!(pending.insertions.is_empty());
        let ev = pending.evictions.get(&hash(1)).unwrap();
        assert_eq!(ev.eviction_version, 200);
        assert_eq!(ev.superseded_version, Some(5));
    }

    #[test]
    fn unrelated_keys_are_independent() {
        let mut updates = HotStateShardUpdates::default();
        updates.insert(hash(1), insertion(Some(5), Some(10)));
        updates.evict(hash(2), eviction(100, Some(7))).unwrap();
        updates.insert(hash(3), insertion(Some(9), Some(30)));

        assert_eq!(
            updates.insertions.get(&hash(1)).unwrap().superseded_version,
            Some(5),
        );
        assert_eq!(
            updates.evictions.get(&hash(2)).unwrap().superseded_version,
            Some(7),
        );
        assert_eq!(
            updates.insertions.get(&hash(3)).unwrap().superseded_version,
            Some(9),
        );
    }
}
