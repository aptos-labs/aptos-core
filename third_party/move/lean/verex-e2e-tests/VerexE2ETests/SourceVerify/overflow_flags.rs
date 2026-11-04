// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn add_flag(a: u8, b: u8) -> bool {
    a.overflowing_add(b).1
}
pub fn subtract_flag(a: u8, b: u8) -> bool {
    a.overflowing_sub(b).1
}
pub fn multiply_flag(a: u16, b: u16) -> bool {
    a.overflowing_mul(b).1
}
pub fn signed_add_flag(a: i8, b: i8) -> bool {
    a.overflowing_add(b).1
}
pub fn signed_subtract_flag(a: i32, b: i32) -> bool {
    a.overflowing_sub(b).1
}
pub fn wrapped_value(a: u8, b: u8) -> u8 {
    a.overflowing_add(b).0
}
pub fn saturating(a: u8, b: u8) -> u8 {
    let (value, overflowed) = a.overflowing_add(b);
    if overflowed { 255 } else { value }
}
pub fn checked(a: u32, b: u32) -> u32 {
    let (value, overflowed) = a.overflowing_add(b);
    if overflowed {
        std::process::abort();
    }
    value
}
pub fn checked_multiply(a: u8, b: u8) -> u8 {
    let (value, overflowed) = a.overflowing_mul(b);
    if overflowed {
        std::process::abort();
    }
    value
}
pub fn pointer_flag(a: usize, b: usize) -> bool {
    a.overflowing_add(b).1
}
pub fn wide_flag(a: u128, b: u128) -> bool {
    a.overflowing_mul(b).1
}
