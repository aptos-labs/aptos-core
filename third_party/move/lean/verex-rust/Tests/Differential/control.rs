// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![feature(core_intrinsics)]
#![allow(internal_features)]

pub fn branch(a: i8, b: i8) -> i8 {
    if a < b {
        a
    } else if a == b {
        0
    } else {
        b
    }
}

pub fn switch(a: u8) -> u8 {
    match a {
        0 => 10,
        1 | 2 => 20,
        3..=9 => 30,
        255 => 40,
        _ => a,
    }
}

pub fn signed_switch(a: i8) -> i8 {
    match a {
        -128 => 1,
        -1 => 2,
        0 => 3,
        127 => 4,
        _ => 5,
    }
}

pub fn early_return(a: u8, b: u8) -> u8 {
    if a == 0 {
        return b;
    }
    if b == 0 {
        return a;
    }
    a & b
}

pub fn sum_to(n: u8) -> u32 {
    let mut total = 0u32;
    let mut i = 0u8;
    while i < n {
        i += 1;
        total += i as u32;
    }
    total
}

pub fn overflowing_sum(n: u8) -> u8 {
    let mut total = 0u8;
    let mut i = 0u8;
    while i < n {
        i += 1;
        total += i;
    }
    total
}

pub fn break_continue(n: u8) -> u8 {
    let mut count = 0u8;
    let mut i = 0u8;
    loop {
        i = core::intrinsics::wrapping_add(i, 1);
        if i % 3 == 0 {
            continue;
        }
        if i >= n {
            break;
        }
        count += 1;
    }
    count
}

pub fn labeled(n: u8, m: u8) -> u16 {
    let mut total = 0u16;
    let mut i = 0u8;
    'outer: while i < n {
        i += 1;
        let mut j = 0u8;
        while j < m {
            j += 1;
            if j == i {
                continue 'outer;
            }
            if total > 100 {
                break 'outer;
            }
            total += 1;
        }
    }
    total
}

pub fn gcd(a: u8, b: u8) -> u8 {
    if b == 0 { a } else { gcd(b, a % b) }
}

pub fn is_even(n: u8) -> bool {
    if n == 0 { true } else { is_odd(n - 1) }
}

pub fn is_odd(n: u8) -> bool {
    if n == 0 { false } else { is_even(n - 1) }
}

pub fn short_circuit(a: u8, b: u8) -> bool {
    a != 0 && 10 / a > b || b == 200
}

pub fn divides(a: u8, b: u8) -> u8 {
    if a == 100 {
        core::intrinsics::abort();
    }
    a / b
}
