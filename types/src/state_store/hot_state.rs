// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// 256 MiB per shard
pub const HOT_STATE_MAX_BYTES_PER_SHARD: usize = 256 * 1024 * 1024;
// 250k items per shard
pub const HOT_STATE_MAX_ITEMS_PER_SHARD: usize = 1_000;
// 10KB, worst case the hot state still caches about 400K items (all shards)
pub const HOT_STATE_MAX_SINGLE_VALUE_BYTES: usize = 10 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LRUEntry<K> {
    /// The key that is slightly newer than the current entry. `None` for the newest entry.
    pub prev: Option<K>,
    /// The key that is slightly older than the current entry. `None` for the oldest entry.
    pub next: Option<K>,
}

impl<K> LRUEntry<K> {
    pub fn uninitialized() -> Self {
        Self {
            prev: None,
            next: None,
        }
    }
}

pub trait THotStateSlot {
    type Key;

    /// Returns the key that is slightly newer in the hot state.
    fn prev(&self) -> Option<&Self::Key>;
    /// Returns the key that is slightly older in the hot state.
    fn next(&self) -> Option<&Self::Key>;

    fn set_prev(&mut self, prev: Option<Self::Key>);
    fn set_next(&mut self, next: Option<Self::Key>);
}
