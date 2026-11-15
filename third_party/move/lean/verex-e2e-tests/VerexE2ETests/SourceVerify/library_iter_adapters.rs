// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Entry {
    pub key: u64,
    pub value: u64,
}

pub fn find_entry(entries: &Vec<Entry>, key: u64) -> usize {
    for (i, entry) in entries.iter().enumerate() {
        if entry.key == key {
            return i;
        }
    }
    entries.len()
}

pub fn last_rank(values: &[u32], default: usize) -> usize {
    for (i, _x) in values.iter().rev().enumerate() {
        return i;
    }
    default
}

pub fn last_of_enumerated(values: &[u32], default: usize) -> usize {
    for (i, _x) in values.iter().enumerate().rev() {
        return i;
    }
    default
}

pub fn last_position(values: &[u32], target: u32) -> usize {
    for (i, x) in values.iter().enumerate().rev() {
        if *x == target {
            return i;
        }
    }
    values.len()
}

pub fn rev_ranks(values: &[u32]) -> usize {
    let mut total = 0;
    for (i, _x) in values.iter().rev().enumerate() {
        total = i + 1;
    }
    total
}
