// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn rev_range_sum(n: u32) -> u32 {
    let mut total = 0;
    for i in (0..n).rev() {
        total += i;
    }
    total
}

pub fn rev_first(n: u32) -> u32 {
    for i in (0..n).rev() {
        return i;
    }
    n
}

pub fn rev_inclusive_count(n: u32) -> u64 {
    let mut total = 0u64;
    for _i in (0..=n).rev() {
        total += 1;
    }
    total
}

pub fn rev_slice_count(values: &[u32]) -> usize {
    let mut n = 0;
    for _x in values.iter().rev() {
        n += 1;
    }
    n
}

pub fn rev_slice_first(values: &[u32], default: u32) -> u32 {
    for x in values.iter().rev() {
        return *x;
    }
    default
}

pub fn enumerate_sum(values: &[u32]) -> usize {
    let mut total = 0;
    for (i, _x) in values.iter().enumerate() {
        total += i;
    }
    total
}

pub fn position(values: &[u32], target: u32) -> usize {
    for (i, x) in values.iter().enumerate() {
        if *x == target {
            return i;
        }
    }
    values.len()
}

pub fn enumerate_range(n: u32) -> u64 {
    let mut total = 0u64;
    for (i, _v) in (0..n).enumerate() {
        total += i as u64;
    }
    total
}
pub fn count_steps(n: u32) -> u32 {
    let mut total = 0;
    for _i in (0..n).step_by(2) {
        total += 1;
    }
    total
}

pub fn first_step(n: u8) -> u8 {
    for i in (0..n).step_by(3) {
        return i;
    }
    n
}

pub fn no_step(n: u32, step: usize) -> u32 {
    let mut iter = (0..n).step_by(step);
    let _ = iter.next();
    n
}

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

pub fn rev_contains(values: &[u32], target: u32) -> bool {
    for x in values.iter().rev() {
        if *x == target {
            return true;
        }
    }
    false
}

pub fn rev_ranks(values: &[u32]) -> usize {
    let mut total = 0;
    for (i, _x) in values.iter().rev().enumerate() {
        total = i + 1;
    }
    total
}

pub fn enumerate_inclusive(n: u32) -> u64 {
    let mut total = 0u64;
    for (_i, _v) in (0..=n).enumerate() {
        total += 1;
    }
    total
}
