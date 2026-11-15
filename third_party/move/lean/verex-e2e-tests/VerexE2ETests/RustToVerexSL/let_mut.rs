// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct P {
    pub a: u32,
}

pub fn reassigned() -> u32 {
    let mut x = 1u32;
    x = 2;
    x
}

pub fn borrowed(v: u32) -> u32 {
    let mut x = 0u32;
    let r = &mut x;
    *r = v;
    x
}

pub fn counted(n: u32) -> u32 {
    let mut i = 0u32;
    while i < n {
        i += 1;
    }
    i
}

pub fn field_written(v: u32) -> u32 {
    let mut p = P { a: 1 };
    p.a = v;
    p.a
}

pub fn field_borrowed(v: u32) -> u32 {
    let mut p = P { a: 1 };
    let q = &mut p.a;
    *q = v;
    p.a
}

pub fn borrowed_in_branch(c: bool) -> u32 {
    let mut x = 0u32;
    if c {
        let r = &mut x;
        *r = 2;
    }
    x
}

pub fn one_binding(v: u32) -> u32 {
    let (mut a, b) = (1u32, v);
    a = b;
    a
}

pub fn repointed(mut a: u32, mut b: u32, c: bool) -> u32 {
    let mut r = &mut a;
    if c {
        r = &mut b;
    }
    *r = 1;
    a
}

pub fn never_written(v: u32) -> u32 {
    let x = v;
    let y = (x, x);
    y.0
}

pub fn assigned_once(c: bool) -> u32 {
    let x;
    if c {
        x = 1u32;
    } else {
        x = 2u32;
    }
    x
}
