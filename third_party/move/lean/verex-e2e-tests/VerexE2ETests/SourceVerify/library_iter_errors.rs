// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn sum_slice(values: &[u32]) -> u64 {
    let mut total = 0u64;
    for x in values.iter() {
        total += *x as u64;
    }
    total
}

pub fn sum_vec(values: &Vec<u32>) -> u64 {
    let mut total = 0u64;
    for x in values.iter() {
        total += *x as u64;
    }
    total
}

pub fn sum_borrowed(values: &[u32]) -> u64 {
    let mut total = 0u64;
    for x in values {
        total += *x as u64;
    }
    total
}

pub fn sum_vec_borrowed(values: &Vec<u32>) -> u64 {
    let mut total = 0u64;
    for x in values {
        total += *x as u64;
    }
    total
}

pub fn count(values: &[u32]) -> usize {
    let mut n = 0;
    for _x in values.iter() {
        n += 1;
    }
    n
}

pub fn contains_value(values: &[u32], target: u32) -> bool {
    for x in values.iter() {
        if *x == target {
            return true;
        }
    }
    false
}

pub fn first_or(values: &[u32], default: u32) -> u32 {
    for x in values.iter() {
        return *x;
    }
    default
}

pub fn last_or(values: &[u32], default: u32) -> u32 {
    let mut last = default;
    for x in values.iter() {
        last = *x;
    }
    last
}
