// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn fill(values: &mut [u32], value: u32) {
    for i in 0..values.len() {
        values[i] = value;
    }
}

pub fn store(values: &mut [u32], position: usize, value: u32) {
    values[position] = value;
}

pub fn set_first(values: &mut [u32], value: u32) -> usize {
    let length = values.len();
    if length > 0 {
        values[0] = value;
    }
    length
}
