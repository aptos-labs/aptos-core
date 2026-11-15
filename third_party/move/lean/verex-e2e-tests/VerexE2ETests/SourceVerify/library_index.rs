// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn element(values: &Vec<u32>, position: usize) -> u32 {
    values[position]
}

pub fn first_pushed(value: u32) -> u32 {
    let mut values = Vec::new();
    values.push(value);
    values[0]
}

pub fn reference(values: &Vec<u32>, position: usize) -> &u32 {
    &values[position]
}

pub fn lookup(values: &Vec<u32>, position: usize) -> u32 {
    match values.get(position) {
        Some(found) => *found,
        None => 0,
    }
}

pub fn found(values: &[u32], position: usize) -> bool {
    values.get(position).is_some()
}
