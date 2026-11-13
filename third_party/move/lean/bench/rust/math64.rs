// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! `aptos_std::math64` in Rust.

pub fn max(a: u64, b: u64) -> u64 {
    if a >= b { a } else { b }
}

pub fn min(a: u64, b: u64) -> u64 {
    if a < b { a } else { b }
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

pub fn mul_div(a: u64, b: u64, c: u64) -> u64 {
    u64::try_from(a as u128 * b as u128 / c as u128).unwrap()
}

pub fn clamp(x: u64, lower: u64, upper: u64) -> u64 {
    x.max(lower).min(upper)
}

pub fn pow(mut n: u64, mut e: u64) -> u64 {
    if e == 0 {
        return 1;
    }
    let mut p = 1;
    while e > 1 {
        if e % 2 == 1 {
            p *= n;
        }
        e /= 2;
        n *= n;
    }
    p * n
}

pub fn floor_log2(mut x: u64) -> u8 {
    // Idiomatic: `assert!(x != 0)`, whose message the exporter rejects.
    if x == 0 {
        std::process::abort();
    }
    let mut res = 0;
    let mut n = 32;
    while n > 0 {
        if x >= (1 << n) {
            x >>= n;
            res += n;
        }
        n >>= 1;
    }
    res
}

pub fn log2(x: u64) -> u64 {
    let integer_part = floor_log2(x);
    let mut y = (if x >= 1 << 32 {
        x >> (integer_part - 32)
    } else {
        x << (32 - integer_part)
    }) as u128;
    let mut frac = 0;
    let mut delta = 1 << 31;
    while delta != 0 {
        y = (y * y) >> 32;
        if y >= (2 << 32) {
            frac += delta;
            y >>= 1;
        }
        delta >>= 1;
    }
    ((integer_part as u64) << 32) + frac
}

pub fn sqrt(x: u64) -> u64 {
    if x == 0 {
        return 0;
    }
    let mut res = 1 << ((floor_log2(x) + 1) >> 1);
    for _ in 0..4 {
        res = (res + x / res) >> 1;
    }
    min(res, x / res)
}

pub fn ceil_div(x: u64, y: u64) -> u64 {
    if x == 0 {
        // Idiomatic: `assert!(y != 0)`.
        if y == 0 {
            std::process::abort();
        }
        0
    } else {
        (x - 1) / y + 1
    }
}
