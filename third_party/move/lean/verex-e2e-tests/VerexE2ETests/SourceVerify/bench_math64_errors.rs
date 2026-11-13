// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// False twins of `bench/rust/math64`: each claim below fails.

pub fn max(a: u64, b: u64) -> u64 {
    if a >= b { a } else { b }
}

pub fn average(a: u64, b: u64) -> u64 {
    if a < b {
        a + (b - a) / 2
    } else {
        b + (a - b) / 2
    }
}

pub fn gcd(a: u64, b: u64) -> u64 {
    let (mut large, mut small) = if a > b { (a, b) } else { (b, a) };
    while small != 0 {
        (large, small) = (small, large % small);
    }
    large
}

pub fn lcm(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 {
        0
    } else {
        a / gcd(a, b) * b
    }
}

pub fn mul_div_overflow(a: u64, b: u64, c: u64) -> u64 {
    u64::try_from(a as u128 * b as u128 / c as u128).unwrap()
}

pub fn clamp(x: u64, lower: u64, upper: u64) -> u64 {
    x.max(lower).min(upper)
}

pub fn ceil_div(x: u64, y: u64) -> u64 {
    if x == 0 {
        if y == 0 {
            std::process::abort();
        }
        0
    } else {
        (x - 1) / y + 1
    }
}
