// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Transferred references and cursors, each iteration count.

pub struct Holder<'a> {
    pub r: &'a mut u8,
}

pub struct Tagged<'a> {
    pub r: &'a mut u8,
    pub n: u8,
}

pub fn step(r: &mut u8) -> &mut u8 {
    r
}

pub fn cursor(a: &mut u8) {
    *a = *a % 4;
    let mut h = Holder { r: a };
    while *h.r != 0 {
        *h.r -= 1;
        let r = h.r;
        h = Holder { r: step(r) };
    }
}

pub fn local_cursor(a: u8) -> u8 {
    let mut x = a % 4;
    let mut h = Holder { r: &mut x };
    while *h.r != 0 {
        *h.r -= 1;
        let r = h.r;
        h = Holder { r: step(r) };
    }
    x
}

pub fn enum_cursor(a: &mut u8) {
    *a = *a % 4;
    let mut cur = Some(a);
    while let Some(r) = cur {
        cur = next(r);
    }
}

pub fn local_enum_cursor(a: u8) -> u8 {
    let mut x = a % 4;
    let mut cur = Some(&mut x);
    while let Some(r) = cur {
        cur = next(r);
    }
    x
}

pub fn transfer_write(a: &mut u8) {
    let mut h = Holder { r: a };
    let r = h.r;
    *r = 7;
    h = Holder { r: step(r) };
    let _ = h;
}

pub fn write_both(a: &mut u8) {
    let mut h = Holder { r: a };
    let r = h.r;
    *r = 3;
    h = Holder { r: step(r) };
    *h.r = 4;
}

pub fn step_direct(a: &mut u8) {
    let mut h = Holder { r: a };
    h = Holder { r: step(h.r) };
    *h.r = 5;
}

pub fn other_field_between(a: &mut u8, k: u8) -> u8 {
    let mut t = Tagged { r: a, n: k };
    let r = t.r;
    let seen = t.n;
    t = Tagged { r: step(r), n: 0 };
    *t.r = seen;
    t.n
}

pub fn next(r: &mut u8) -> Option<&mut u8> {
    if *r == 0 {
        None
    } else {
        *r -= 1;
        Some(r)
    }
}

pub fn return_between(c: bool, a: &mut u8) {
    let mut h = Holder { r: a };
    let r = h.r;
    *r = 1;
    if c {
        return;
    }
    h = Holder { r: step(r) };
    *h.r = 2;
}
