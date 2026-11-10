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

pub fn or_default_some() -> u32 {
    let value = Some(3u32);
    value.unwrap_or(7)
}

pub fn must(value: Option<u32>) -> u32 {
    value.unwrap()
}

pub fn must_some() -> u32 {
    let value = Some(3u32);
    value.unwrap()
}

pub fn present(value: Option<u32>) -> bool {
    value.is_some()
}

pub fn absent(value: &Option<u32>) -> bool {
    value.is_none()
}

pub fn ok_or_default(value: Result<u32, u8>) -> u32 {
    value.unwrap_or(7)
}

pub fn must_ok(value: Result<u32, u8>) -> u32 {
    value.unwrap()
}

pub fn succeeded(value: Result<u32, u8>) -> bool {
    value.is_ok()
}

pub fn failed(value: &Result<u32, u8>) -> bool {
    value.is_err()
}
