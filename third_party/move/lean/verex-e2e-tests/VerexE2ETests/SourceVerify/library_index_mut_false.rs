// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn store(values: &mut Vec<u32>, position: usize, value: u32) {
    values[position] = value;
}

pub fn unguarded(values: &mut Vec<u32>, position: usize, value: u32) {
    values[position] = value;
}

pub fn other_position(values: &mut Vec<u32>, position: usize, value: u32) {
    values[position] = value;
}
