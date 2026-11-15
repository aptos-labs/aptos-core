// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn guarded(_flag: bool, x: u64) -> u64 {
    x
}

pub fn two_flags(_a: bool, _b: bool, x: u64) -> u64 {
    x
}


pub fn count_inclusive(n: u32) -> u64 {
    let mut total = 0u64;
    for _i in 0..=n {
        total += 1;
    }
    total
}
