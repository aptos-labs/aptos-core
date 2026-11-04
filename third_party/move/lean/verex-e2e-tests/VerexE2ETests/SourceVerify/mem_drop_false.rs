// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A wrong postcondition fails on each.

pub struct Holder<'a> {
    pub r: &'a mut u8,
}

pub fn value(x: u8) -> u8 {
    drop(x);
    x
}

pub fn owned(v: Vec<u32>, n: u32) -> u32 {
    drop(v);
    n
}

pub fn ends_holder(a: u8) -> u8 {
    let mut x = a;
    let h = Holder { r: &mut x };
    *h.r = a % 100 + 1;
    drop(h);
    x
}

pub fn ends_reference(a: u8) -> u8 {
    let mut x = a;
    let r = &mut x;
    *r = a % 100 + 2;
    drop(r);
    x
}

pub fn generic<T>(x: T) {
    drop(x);
}

pub fn lets_writer_go(mut a: u8) -> u8 {
    let r = &mut a;
    *r = 5;
    core::mem::drop(r);
    a = a % 100 + 1;
    a
}
