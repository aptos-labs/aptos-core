// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn add_flag(a: u8, b: u8) -> bool {
    a.overflowing_add(b).1
}
pub fn signed_add_flag(a: i8, b: i8) -> bool {
    a.overflowing_add(b).1
}
pub fn unguarded(a: u8, b: u8) -> u8 {
    let (value, overflowed) = a.overflowing_add(b);
    if overflowed {
        std::process::abort();
    }
    value
}
