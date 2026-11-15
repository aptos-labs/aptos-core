// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::cmp::Ordering;

pub fn less(a: u64, b: u64) -> bool {
    match a.cmp(&b) {
        Ordering::Less => true,
        _ => false,
    }
}

#[repr(i8)]
pub enum Sign {
    Negative = -1,
    Zero = 0,
    Positive = 1,
}

pub fn sign(a: i64) -> Sign {
    if a < 0 {
        Sign::Negative
    } else if a == 0 {
        Sign::Zero
    } else {
        Sign::Positive
    }
}
