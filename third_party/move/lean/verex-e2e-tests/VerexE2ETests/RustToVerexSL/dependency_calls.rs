// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn checked(left: u32, right: u32) -> Option<u32> {
    left.checked_add(right)
}

pub fn or_zero(value: Option<u32>) -> u32 {
    value.unwrap_or(0)
}

pub fn length(values: &Vec<u32>) -> usize {
    values.len()
}

pub fn push_one(values: &mut Vec<u32>) {
    values.push(1)
}

pub fn copied(value: &Option<u32>) -> Option<u32> {
    value.clone()
}
