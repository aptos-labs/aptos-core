// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn head(values: &[u32]) -> Option<&u32> {
    values.first()
}

pub fn tail(values: &[u32]) -> Option<&u32> {
    values.last()
}

pub fn parts(values: &[u32], mid: usize) -> usize {
    let (left, right) = values.split_at(mid);
    left.len() + 2 * right.len()
}

pub fn flip(values: &mut [u32]) {
    values.reverse();
}

pub fn has(values: &[u32], value: u32) -> bool {
    values.contains(&value)
}

pub fn vec_head(values: &Vec<u32>) -> Option<&u32> {
    values.first()
}
