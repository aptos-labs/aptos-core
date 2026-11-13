// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A closure with a specification is a target of its own, and its callers use the
// contract.

pub fn specified(c: u32) -> u32 {
    let add = |a: u32| a + c;
    add(1)
}

pub fn specified_false(c: u32) -> u32 {
    let add = |a: u32| a + c;
    add(1)
}

pub fn caller_false(c: u32) -> u32 {
    let add = |a: u32| a + c;
    add(1)
}

pub fn body_false(c: u32) -> u32 {
    let add = |a: u32| a + c;
    add(1)
}

pub fn panicking(c: u32) -> u32 {
    let add = |a: u32| a + c;
    add(1)
}

pub fn looped(n: u32) -> u32 {
    let sum = |m: u32| {
        let mut s = 0;
        let mut i = 0;
        while i < m {
            s = s + 2;
            i = i + 1;
        }
        s
    };
    sum(n)
}

pub fn looped_false(n: u32) -> u32 {
    let sum = |m: u32| {
        let mut s = 0;
        let mut i = 0;
        while i < m {
            s = s + 2;
            i = i + 1;
        }
        s
    };
    sum(n)
}

// The enclosing function's first loop is its own, not the closure's.
pub fn loops_apart(n: u32) -> u32 {
    let sum = |m: u32| {
        let mut s = 0;
        let mut i = 0;
        while i < m {
            s = s + 2;
            i = i + 1;
        }
        s
    };
    let mut total = 0;
    let mut j = 0;
    while j < n {
        total = total + 1;
        j = j + 1;
    }
    total + sum(0)
}

pub fn mut_specified(n0: u32) -> u32 {
    let mut n = n0;
    let mut inc = |d: u32| {
        n = n + d;
    };
    inc(1);
    n
}

pub fn mut_specified_false(n0: u32) -> u32 {
    let mut n = n0;
    let mut inc = |d: u32| {
        n = n + d;
    };
    inc(1);
    n
}

pub fn proved(c: u32) -> u32 {
    let same = || c;
    same()
}
