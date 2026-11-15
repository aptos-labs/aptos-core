// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn unguarded(values: &Vec<u32>, position: usize) -> u32 {
    values[position]
}

pub fn wrong_element(values: &Vec<u32>) -> u32 {
    values[0]
}

pub fn wrong_lookup(values: &Vec<u32>, position: usize) -> u32 {
    match values.get(position) {
        Some(found) => *found,
        None => 0,
    }
}

pub fn from_range(values: &Vec<u32>) -> &[u32] {
    &values[1..]
}
