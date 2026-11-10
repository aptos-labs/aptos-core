// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn insert_at(values: &mut Vec<u32>, at: usize, value: u32) {
    values.insert(at, value);
}

pub fn remove_at(values: &mut Vec<u32>, at: usize) -> u32 {
    values.remove(at)
}

pub fn keep_two(values: &mut Vec<u32>) {
    values.truncate(2);
}

pub fn clear_all(values: &mut Vec<u32>) {
    values.clear();
}

pub fn whole(values: &Vec<u32>) -> &[u32] {
    values.as_slice()
}

pub fn append(values: &mut Vec<u32>, other: &[u32]) {
    values.extend_from_slice(other);
}

pub fn filled(count: usize) -> Vec<u32> {
    vec![7u32; count]
}
