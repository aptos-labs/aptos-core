// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn or_panic(value: Option<u32>) -> u32 {
    value.expect("a value")
}

pub fn view(value: &Option<u32>) -> Option<&u32> {
    value.as_ref()
}

pub fn or_error(value: Option<u32>, error: u8) -> Result<u32, u8> {
    value.ok_or(error)
}

pub fn taken(value: &mut Option<u32>) -> Option<u32> {
    value.take()
}

pub fn replaced(value: &mut Option<u32>, next: u32) -> Option<u32> {
    value.replace(next)
}

pub fn defaulted(value: Option<u32>) -> u32 {
    value.unwrap_or_default()
}
