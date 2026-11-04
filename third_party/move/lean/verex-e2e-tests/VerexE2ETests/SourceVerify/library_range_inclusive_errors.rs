// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn count_inclusive(n: u32) -> u64 {
    let mut total = 0u64;
    for _i in 0..=n {
        total += 1;
    }
    total
}

pub fn count_to_max(n: u8) -> u32 {
    let mut total = 0u32;
    for _i in n..=255 {
        total += 1;
    }
    total
}

pub fn sum_inclusive(n: u32) -> u32 {
    let mut total = 0;
    for i in 0..=n {
        total += i;
    }
    total
}

pub fn last_inclusive(a: u8, b: u8) -> u8 {
    let mut last = 0;
    for i in a..=b {
        last = i;
    }
    last
}

pub fn empty_inclusive(a: u32, b: u32) -> bool {
    for _i in a..=b {
        return false;
    }
    true
}

pub fn first_inclusive(a: u32, b: u32) -> u32 {
    for i in a..=b {
        return i;
    }
    b
}
