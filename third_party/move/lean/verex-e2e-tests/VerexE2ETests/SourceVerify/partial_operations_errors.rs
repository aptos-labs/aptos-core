// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![feature(core_intrinsics)]
#![allow(internal_features)]

pub fn divide(a: u32, b: u32) -> u32 {
    unsafe { core::intrinsics::unchecked_div(a, b) }
}

pub fn overflow_value(a: u8, b: u8) -> u8 {
    let (value, _) = a.overflowing_add(b);
    value
}
