// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::cmp::Ordering;

pub fn less_u64(a: u64, b: u64) -> bool {
    match a.cmp(&b) {
        Ordering::Less => true,
        _ => false,
    }
}

pub fn order_i8(a: i8, b: i8) -> i8 {
    match a.cmp(&b) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

pub fn order_usize(a: usize, b: usize) -> u8 {
    match a.cmp(&b) {
        Ordering::Less => 0,
        Ordering::Equal => 1,
        Ordering::Greater => 2,
    }
}

pub fn pick_u128(a: u128, b: u128) -> u128 {
    match a.cmp(&b) {
        Ordering::Greater => a,
        _ => b,
    }
}
