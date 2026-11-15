// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn push_two() -> Vec<u32> {
    let mut values = Vec::new();
    values.push(1);
    values.push(2);
    values
}

pub fn length(values: &Vec<u32>) -> usize {
    values.len()
}

pub fn push_then_length(values: &mut Vec<u32>, value: u32) -> usize {
    values.push(value);
    values.len()
}

pub fn empty(values: &Vec<u32>) -> bool {
    values.is_empty()
}

pub fn pop_one(values: &mut Vec<u32>) -> Option<u32> {
    values.pop()
}

pub fn pop_after_push(values: &mut Vec<u32>, value: u32) -> u32 {
    values.push(value);
    match values.pop() {
        Some(last) => last,
        None => 0,
    }
}

pub fn pop_from_empty() -> bool {
    let mut values: Vec<u32> = Vec::new();
    values.pop().is_none()
}

pub fn pass(values: Vec<u32>) -> Vec<u32> {
    values
}

pub fn dropped(_values: Vec<u32>) -> u32 {
    1
}

pub fn as_slice(values: &Vec<u32>) -> &[u32] {
    values
}

pub fn slice_length(values: &Vec<u32>) -> usize {
    let slice: &[u32] = values;
    slice.len()
}
