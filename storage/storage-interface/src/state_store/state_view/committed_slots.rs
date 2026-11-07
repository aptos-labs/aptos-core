// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{
    state_update_refs::StateUpdateRefs, state_view::cached_state_view::ShardedStateCache,
};
use aptos_types::{
    state_store::{state_key::StateKey, state_slot::StateSlot},
    transaction::Version,
};
use dashmap::DashMap;
use parking_lot::Mutex;
use rayon::prelude::*;

/// The slot of every key a run of consecutive blocks read or wrote, as the
/// last of them left it. A block that reads such a key from the execution
/// pool never asks the state view for it, so its memorized reads lack the
/// old slot the state update needs; this answers for those keys without a
/// lookup through the state layers or the DB.
pub struct CommittedSlots {
    slots: DashMap<StateKey, StateSlot, ahash::RandomState>,
    /// The version right after the last block absorbed; `None` until one is.
    next_version: Mutex<Option<Version>>,
    max_keys: usize,
}

impl CommittedSlots {
    pub fn new(max_keys: usize) -> Self {
        Self {
            slots: DashMap::with_hasher(ahash::RandomState::new()),
            next_version: Mutex::new(None),
            max_keys,
        }
    }

    /// The slots as the block starting at `first_version` sees them, or
    /// `None` when the absorbed blocks do not lead up to it.
    pub fn for_block(&self, first_version: Version) -> Option<&Self> {
        if *self.next_version.lock() == Some(first_version) {
            Some(self)
        } else {
            self.clear();
            None
        }
    }

    /// Whether a key is hot is the hot-state LRU's to say when the slot is
    /// consulted, which it only is for keys the LRU does not have; a slot
    /// recorded as hot would contradict it once evicted.
    fn cold(slot: StateSlot) -> StateSlot {
        if slot.is_hot() {
            slot.to_cold()
        } else {
            slot
        }
    }

    pub fn get(&self, key: &StateKey) -> Option<StateSlot> {
        self.slots.get(key).map(|slot| slot.clone())
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub fn clear(&self) {
        self.slots.clear();
        *self.next_version.lock() = None;
    }

    /// Records a block: what it read from the state view, which the pool has
    /// from now on, then what it wrote. The block must start where the
    /// absorbed blocks end, otherwise the record starts over from it.
    pub fn absorb(&self, reads: &ShardedStateCache, updates: &StateUpdateRefs) {
        let mut next_version = self.next_version.lock();
        if *next_version != Some(updates.per_version.first_version) {
            self.slots.clear();
        }
        reads.shards.par_iter().for_each(|shard| {
            for entry in shard.iter() {
                self.slots
                    .insert(entry.key().clone(), Self::cold(entry.value().clone()));
            }
        });
        for batch in [
            updates.for_last_checkpoint_batched(),
            updates.for_latest_batched(),
        ]
        .into_iter()
        .flatten()
        {
            batch.shards.par_iter().for_each(|shard| {
                for (key, update) in shard {
                    if let Some(slot) = update.to_result_slot((*key).clone()) {
                        self.slots.insert((*key).clone(), Self::cold(slot));
                    }
                }
            });
        }
        if self.slots.len() > self.max_keys {
            self.slots.clear();
            *next_version = None;
        } else {
            *next_version = Some(
                updates.per_version.first_version + updates.per_version.num_versions as Version,
            );
        }
    }
}
