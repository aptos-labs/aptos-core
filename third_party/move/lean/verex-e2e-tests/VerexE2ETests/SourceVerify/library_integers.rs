// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn shift_past_width(value: u32) -> u32 {
    value.wrapping_shl(33)
}

pub fn shift_right_past_width(value: u32) -> u32 {
    value.wrapping_shr(35)
}

pub fn signed_shift_past_width(value: i32) -> i32 {
    value.wrapping_shl(33)
}

pub fn signed_shift_right_past_width(value: i32) -> i32 {
    value.wrapping_shr(33)
}

pub fn wide_shift(value: u64, distance: u32) -> u64 {
    value.wrapping_shl(distance)
}

pub fn saturate_up(left: u8, right: u8) -> u8 {
    left.saturating_add(right)
}

pub fn saturate_down(left: i8, right: i8) -> i8 {
    left.saturating_sub(right)
}

pub fn multiply(left: u64, right: u64) -> Option<u64> {
    left.checked_mul(right)
}

pub fn subtract(left: usize, right: usize) -> Option<usize> {
    left.checked_sub(right)
}

pub fn signed_add(left: i16, right: i16) -> Option<i16> {
    left.checked_add(right)
}

pub fn wrapped(left: u32, right: u32) -> u32 {
    left.wrapping_add(right)
}
