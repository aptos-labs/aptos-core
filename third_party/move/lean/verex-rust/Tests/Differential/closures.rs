// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn direct(c: u8, a: u8) -> u8 {
    let g = |x: u8| x + c;
    g(a)
}

pub fn twice(c: u8, a: u8) -> u8 {
    let g = |x: u8| x / c;
    g(a) + g(a)
}

pub fn no_upvars(a: u8, b: u8) -> u8 {
    let g = |x: u8, y: u8| x % y;
    g(a, b)
}

pub fn counter(n: u8, d: u8) -> u8 {
    let mut count = n;
    let mut inc = |x: u8| {
        count += x;
    };
    inc(d);
    inc(d);
    count
}

pub fn moved(n: u8, d: u8) -> (u8, u8) {
    let mut count = n;
    let mut next = move || {
        count += d;
        count
    };
    let first = next();
    (first, count)
}

pub fn once(n: (u8, u8)) -> u8 {
    let take = move || n.0 + n.1;
    take()
}

pub fn looped(a: u8) -> u8 {
    let sum = |m: u8| {
        let mut s = 0u8;
        let mut i = 0u8;
        while i < m % 8 {
            s += i;
            i += 1;
        }
        s
    };
    sum(a)
}

pub fn nested(a: u8, b: u8) -> u8 {
    let outer = |x: u8| {
        let inner = |y: u8| y + x + b;
        inner(1)
    };
    outer(a)
}

pub fn write_through(a: &mut u8, b: u8) {
    let mut write = |x: u8| {
        *a = x;
    };
    write(b);
}
