// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn shift_past_width(value: u32) -> u32 {
    value.wrapping_shl(33)
}

pub fn saturate_up(left: u8, right: u8) -> u8 {
    left.saturating_add(right)
}

pub fn subtract(left: usize, right: usize) -> Option<usize> {
    left.checked_sub(right)
}
