// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn truncate(a: u32) -> u8 {
    a as u8
}
pub fn truncate_signed(a: u32) -> i8 {
    a as i8
}
pub fn narrow_signed(a: i32) -> i16 {
    a as i16
}
pub fn to_unsigned(a: i16) -> u16 {
    a as u16
}
pub fn sign_extend(a: i16) -> i32 {
    a as i32
}
pub fn sign_extend_unsigned(a: i16) -> u32 {
    a as u32
}
pub fn low_byte_is_zero(a: u32) -> bool {
    (a as u8) == 0
}
pub fn branch_on_truncated(a: u32) -> u8 {
    if (a as u8) > 100 { 1 } else { 0 }
}
pub fn branch_on_sign(a: u32) -> bool {
    if (a as i8) < 0 { true } else { false }
}
pub fn round_trip(a: u8) -> u8 {
    (a as u32) as u8
}
pub fn panic_on_truncation(a: u32) -> u8 {
    let low = a as u8;
    if (low as u32) != a {
        std::process::abort();
    }
    low
}
