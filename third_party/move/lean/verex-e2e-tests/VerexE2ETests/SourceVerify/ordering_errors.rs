// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::cmp::Ordering;

pub fn comparator(a: u64, b: u64) -> bool {
    match a.cmp(&b) {
        Ordering::Less => true,
        _ => false,
    }
}

pub fn order(a: i64, b: i64) -> i64 {
    match a.cmp(&b) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}
