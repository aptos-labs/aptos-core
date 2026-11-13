// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn store(values: &mut Vec<u32>, position: usize, value: u32) {
    values[position] = value;
}

pub fn bump(values: &mut Vec<u32>, position: usize) {
    values[position] += 1;
}

pub fn swap_first(values: &mut Vec<u32>, value: u32) -> u32 {
    let old = values[0];
    values[0] = value;
    old
}

pub fn fill(values: &mut Vec<u32>, value: u32) {
    for i in 0..values.len() {
        values[i] = value;
    }
}
