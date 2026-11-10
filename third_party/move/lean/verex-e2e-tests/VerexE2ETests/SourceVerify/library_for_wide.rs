// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn count(n: u64) -> u64 {
    let mut total = 0;
    for _i in 0..n {
        total += 1;
    }
    total
}

pub fn count_signed(n: i64) -> i64 {
    let mut total = 0;
    for _i in 0..n {
        total += 1;
    }
    total
}

pub fn both(values: &[u32], n: u64) -> u64 {
    let mut total = 0;
    for _i in 0..values.len() {
        total += 1;
    }
    for _j in 0..n {
        total += 1;
    }
    total
}
