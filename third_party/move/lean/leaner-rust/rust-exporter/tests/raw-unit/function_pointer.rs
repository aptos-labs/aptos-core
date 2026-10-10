// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[inline(never)]
pub fn increment(value: u32) -> u32 {
    value ^ 3
}

pub fn apply(value: u32) -> u32 {
    let function: fn(u32) -> u32 = increment;
    function(value)
}
