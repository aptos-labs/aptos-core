// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn magnitude(value: i32) -> i32 {
    value.abs()
}

pub fn quotient(left: u32, right: u32) -> Option<u32> {
    left.checked_div(right)
}

pub fn signed_quotient(left: i32, right: i32) -> Option<i32> {
    left.checked_div(right)
}

pub fn wrapped(left: u32, right: u32) -> u32 {
    left.rem_euclid(right)
}

pub fn signed_wrapped(left: i32, right: i32) -> i32 {
    left.rem_euclid(right)
}

pub fn single_bit(value: u8) -> bool {
    value.is_power_of_two()
}

pub fn high_zeros(value: u8) -> u32 {
    value.leading_zeros()
}

pub fn low_zeros(value: u8) -> u32 {
    value.trailing_zeros()
}
