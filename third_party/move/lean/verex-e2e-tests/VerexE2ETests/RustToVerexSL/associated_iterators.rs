// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn sum_slice(values: &[u32]) -> u64 {
    let mut total = 0;
    for x in values.iter() {
        total += *x as u64;
    }
    total
}

pub fn sum_vec(values: &Vec<u32>) -> u64 {
    let mut total = 0;
    for x in values {
        total += *x as u64;
    }
    total
}

pub fn inclusive(n: u32) -> u32 {
    let mut total = 0;
    for i in 0..=n {
        total += i;
    }
    total
}

pub fn reversed(n: u32) -> u32 {
    let mut total = 0;
    for i in (0..n).rev() {
        total += i;
    }
    total
}

pub fn position(values: &[u32], target: u32) -> usize {
    for (i, x) in values.iter().enumerate() {
        if *x == target {
            return i;
        }
    }
    values.len()
}

pub fn stepped(n: u32) -> u32 {
    let mut total = 0;
    for i in (0..n).step_by(2) {
        total += i;
    }
    total
}
