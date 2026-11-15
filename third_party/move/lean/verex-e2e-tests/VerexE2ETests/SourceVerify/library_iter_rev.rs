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

pub fn rev_contains(values: &[u32], target: u32) -> bool {
    for x in values.iter().rev() {
        if *x == target {
            return true;
        }
    }
    false
}
