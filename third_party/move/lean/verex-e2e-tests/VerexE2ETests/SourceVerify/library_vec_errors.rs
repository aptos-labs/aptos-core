// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Guard {
    pub value: u32,
}

impl Drop for Guard {
    fn drop(&mut self) {}
}

pub fn push_one(values: &mut Vec<u32>) {
    values.push(1)
}

pub fn pop_some(values: &mut Vec<u32>) -> Option<u32> {
    values.pop()
}

pub fn drop_guards(_values: Vec<Guard>) -> u32 {
    1
}

pub fn deduped(values: &mut Vec<u32>) {
    values.dedup()
}
