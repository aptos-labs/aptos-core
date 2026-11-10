// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn keep(values: Vec<u32>) -> Vec<u32> {
    values
}

pub fn borrowed(_values: &Vec<u32>) -> u32 {
    1
}

pub fn length(values: &[u32]) -> usize {
    values.len()
}

pub fn named(value: String) -> String {
    value
}

pub fn view(value: &str) -> usize {
    value.len()
}
