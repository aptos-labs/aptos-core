// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_update_refs::StateUpdateRefs;
use aptos_types::{
    state_store::{state_key::StateKey, state_slot::StateSlot},
    transaction::Version,
};
use dashmap::DashMap;
use parking_lot::Mutex;
use rayon::prelude::*;

/// The last write to every key a run of consecutive blocks made, as the slot
/// it left behind. A block that read such a key from the execution pool
/// never asked the state view for it, so its memorized reads lack the old
/// slot the state update needs; this answers for those keys without a
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

    /// Records a block's writes. The block must start where the absorbed
    /// blocks end, otherwise the record starts over from this block.
    pub fn absorb(&self, updates: &StateUpdateRefs) {
        let mut next_version = self.next_version.lock();
        if *next_version != Some(updates.per_version.first_version) {
            self.slots.clear();
        }
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
                        self.slots.insert((*key).clone(), slot);
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
