// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LRUEntry<K> {
    /// The key that is slightly newer than the current entry. `None` for the newest entry.
    pub prev: Option<K>,
    /// The key that is slightly older than the current entry. `None` for the oldest entry.
    pub next: Option<K>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeculativeLRUEntry<V> {
    Existing(V),
    Deleted,
}
