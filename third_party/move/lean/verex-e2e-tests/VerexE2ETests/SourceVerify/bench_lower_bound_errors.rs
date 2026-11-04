// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// False twins of `bench/rust/lower_bound`: each claim below fails.

pub struct Entry<V> {
    pub key: u64,
    pub value: V,
}

pub fn unsorted<V>(key: u64, entries: &[Entry<V>], start: usize, end: usize) -> usize {
    let mut low = start;
    let mut high = end;
    while low < high {
        let mid = low + (high - low) / 2;
        if entries[mid].key < key {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}

pub fn strict<V>(key: u64, entries: &[Entry<V>], start: usize, end: usize) -> usize {
    let mut low = start;
    let mut high = end;
    while low < high {
        let mid = low + (high - low) / 2;
        if entries[mid].key < key {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}

pub fn past_end<V>(key: u64, entries: &[Entry<V>], start: usize, end: usize) -> usize {
    let mut low = start;
    let mut high = end;
    while low < high {
        let mid = low + (high - low) / 2;
        if entries[mid].key < key {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}

pub fn wrong_invariant<V>(key: u64, entries: &[Entry<V>], start: usize, end: usize) -> usize {
    let mut low = start;
    let mut high = end;
    while low < high {
        let mid = low + (high - low) / 2;
        if entries[mid].key < key {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}
