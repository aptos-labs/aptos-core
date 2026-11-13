// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn divide(dividend: u32, divisor: u32) -> u32 {
    dividend / divisor
}

pub fn guarded(dividend: u32, divisor: u32) -> u32 {
    if divisor == 0 {
        0
    } else {
        dividend / divisor
    }
}

pub fn permitted(dividend: u32, divisor: u32) -> u32 {
    dividend / divisor
}

pub fn wrong_permission(dividend: u32, divisor: u32) -> u32 {
    dividend / divisor
}

pub fn sum(left: u32, right: u32) -> u32 {
    left + right
}

pub fn calls_divide(dividend: u32) -> u32 {
    guarded(dividend, 0) + divide(dividend, dividend)
}

pub fn two_divisions(a: u32, b: u32, c: u32) -> u32 {
    if a > 10 {
        a / b
    } else {
        c / b + c / a
    }
}

pub fn routed(a: u32, b: u32) -> u32 {
    a + 1 + b
}

pub fn unrouted(a: u32) -> u32 {
    a + 1
}

pub fn nonzero(a: u32) -> u32 {
    100 / a
}

pub fn unspecified(a: u32) -> u32 {
    if a > 0 {
        nonzero(a)
    } else {
        100 / a
    }
}

pub fn requested(a: u32) -> u32 {
    a + 1
}

pub fn in_loop(n: u32, d: u32) -> u32 {
    let mut i = 0;
    let mut s = 100;
    while i < n {
        s = s / d;
        i = i + 1;
    }
    s
}
