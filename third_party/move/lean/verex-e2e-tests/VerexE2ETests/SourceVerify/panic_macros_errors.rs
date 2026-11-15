// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

const LIMIT: u64 = 100;

static BASE: u64 = 7;

pub fn plain_assert(c: u64) -> u64 {
    assert!(c != 0);
    c
}

pub fn message_assert(c: u64, x: u64) -> u64 {
    assert!(c != 0, "c is {} with {}", c, x);
    c
}

pub fn literal_message_assert(c: u64) -> u64 {
    assert!(c != 0, "c is zero");
    c
}

pub fn literal_panic(c: u64) -> u64 {
    if c == 0 {
        panic!("zero");
    }
    c
}

pub fn format_panic(c: u64, x: u64) -> u64 {
    if c == 0 {
        panic!("zero with {}", x);
    }
    c
}

pub fn unreachable_arm(c: u64) -> u64 {
    if c > 0 {
        c
    } else {
        unreachable!()
    }
}

pub fn debug_only(c: u64) -> u64 {
    debug_assert!(c != 0);
    c
}

pub fn expect_option(value: Option<u64>) -> u64 {
    value.expect("a value")
}

pub fn unwrap_option(value: Option<u64>) -> u64 {
    value.unwrap()
}

pub fn expect_result(value: Result<u64, u64>) -> u64 {
    value.expect("a value")
}

pub fn unwrap_result(value: Result<u64, u64>) -> u64 {
    value.unwrap()
}

pub fn static_read() -> u64 {
    BASE
}

pub fn const_limit(c: u64) -> u64 {
    assert!(c < LIMIT);
    LIMIT - c
}

pub fn maximum_guard(c: u64) -> u64 {
    assert!(c < u64::MAX);
    c + 1
}

pub fn always() -> u64 {
    panic!("always")
}

pub fn past_the_panic(c: u64) -> u64 {
    assert!(c != 0);
    c
}

pub fn not_always(c: u64) -> u64 {
    assert!(c != 0);
    c
}

pub fn join_assert(a: bool, b: u64, c: u64) -> u64 {
    let mut x = 0;
    if a {
        x = b / 2;
    } else {
        x = b / 3;
    }
    assert!(c != 0, "c is zero");
    x
}

pub fn join_panic(a: bool, b: u64) -> u64 {
    let mut x = 0;
    if a {
        x = b / 2;
    } else {
        x = b / 3;
    }
    if x > 100 {
        return x;
    }
    panic!("small")
}

pub fn unguarded() -> u64 {
    panic!("unguarded")
}
