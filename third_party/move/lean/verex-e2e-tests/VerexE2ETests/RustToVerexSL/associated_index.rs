// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn element(values: &Vec<u32>, index: usize) -> u32 {
    values[index]
}

pub fn first(values: &Vec<u32>) -> u32 {
    values[0]
}

pub fn reference(values: &Vec<u32>, index: usize) -> &u32 {
    &values[index]
}

pub fn store(values: &mut Vec<u32>, index: usize, value: u32) {
    values[index] = value;
}
