// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn add_or_zero(left: u32, right: u32) -> u32 {
    match left.checked_add(right) {
        Some(sum) => sum,
        None => 0,
    }
}

pub fn or_default(value: Option<u32>) -> u32 {
    value.unwrap_or(7)
}

pub fn must_none() -> u32 {
    let value: Option<u32> = None;
    value.unwrap()
}

pub fn must_unstated(value: Option<u32>) -> u32 {
    value.unwrap()
}

pub fn present(value: Option<u32>) -> bool {
    value.is_some()
}

pub fn must_err() -> u32 {
    let value: Result<u32, u8> = Err(1);
    value.unwrap()
}
