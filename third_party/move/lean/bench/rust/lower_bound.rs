// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! The binary search of `aptos_framework::ordered_map` in Rust.

pub struct Entry<V> {
    pub key: u64,
    pub value: V,
}

/// The first position in `entries[start..end]` whose key is not less than `key`.
pub fn binary_search<V>(key: u64, entries: &[Entry<V>], start: usize, end: usize) -> usize {
    let mut low = start;
    let mut high = end;
    while low < high {
        let mid = low + (high - low) / 2;
        // Idiomatic: `match entries[mid].key.cmp(&key)`.
        if entries[mid].key < key {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}

/// `binary_search` over a whole sorted `Vec`.
pub fn lower_bound<V>(entries: &Vec<Entry<V>>, key: u64) -> usize {
    binary_search(key, entries, 0, entries.len())
}
