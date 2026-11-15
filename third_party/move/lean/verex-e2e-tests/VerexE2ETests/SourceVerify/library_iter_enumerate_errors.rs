// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn enumerate_sum(values: &[u32]) -> usize {
    let mut total = 0;
    for (i, _x) in values.iter().enumerate() {
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

pub fn enumerate_range(n: u32) -> u64 {
    let mut total = 0u64;
    for (i, _v) in (0..n).enumerate() {
        total += i as u64;
    }
    total
}

pub fn count_steps(n: u32) -> u32 {
    let mut total = 0;
    for _i in (0..n).step_by(2) {
        total += 1;
    }
    total
}

pub fn first_step(n: u8) -> u8 {
    for i in (0..n).step_by(3) {
        return i;
    }
    n
}

pub fn no_step(n: u32, step: usize) -> u32 {
    let mut iter = (0..n).step_by(step);
    let _ = iter.next();
    n
}

pub fn enumerate_inclusive(n: u32) -> u64 {
    let mut total = 0u64;
    for (_i, _v) in (0..=n).enumerate() {
        total += 1;
    }
    total
}
