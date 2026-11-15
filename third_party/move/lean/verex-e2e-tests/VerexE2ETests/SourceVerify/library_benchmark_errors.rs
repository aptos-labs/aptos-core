// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// False claims over the contracts of `library_benchmark.rs`.

pub fn larger(a: u64, b: u64) -> u64 {
    a.max(b)
}

pub fn smaller(a: i32, b: i32) -> i32 {
    a.min(b)
}

pub fn narrow(value: u128) -> u64 {
    u64::try_from(value).unwrap()
}

pub fn repeated(length: usize) -> Vec<bool> {
    vec![true; length]
}

pub fn take(values: &mut Vec<u32>, position: usize) -> u32 {
    values.swap_remove(position)
}

pub fn keep_order(values: &mut Vec<u32>, position: usize) -> u32 {
    values.swap_remove(position)
}

pub fn exchange(values: &mut Vec<u32>, first: usize, second: usize) {
    values.swap(first, second);
}

pub fn clear(values: &mut Vec<bool>) {
    values.fill(false);
}
