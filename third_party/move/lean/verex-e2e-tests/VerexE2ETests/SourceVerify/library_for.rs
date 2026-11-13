// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn count(n: u32) -> u32 {
    let mut total = 0;
    for _i in 0..n {
        total += 1;
    }
    total
}

pub fn sum(n: u32) -> u32 {
    let mut total = 0;
    for i in 0..n {
        total += i;
    }
    total
}

pub fn find_vec(values: &Vec<u32>, target: u32) -> bool {
    for i in 0..values.len() {
        if values[i] == target {
            return true;
        }
    }
    false
}

pub fn find_slice(values: &[u32], target: u32) -> bool {
    for i in 0..values.len() {
        if values[i] == target {
            return true;
        }
    }
    false
}

pub fn nested(n: u32, m: u32) -> u32 {
    let mut total = 0;
    for _i in 0..n {
        for _j in 0..m {
            total += 1;
        }
    }
    total
}
