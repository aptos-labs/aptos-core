// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct P {
    pub a: u32,
}

pub fn reassigned(v: u32) -> u32 {
    let mut x = 1u32;
    x = v;
    x
}

pub fn borrowed(v: u32) -> u32 {
    let mut x = 0u32;
    let r = &mut x;
    *r = v;
    x
}

pub fn field_borrowed(v: u32) -> u32 {
    let mut p = P { a: 1 };
    let q = &mut p.a;
    *q = v;
    p.a
}

pub fn in_branch(c: bool, v: u32) -> u32 {
    let mut x = 0u32;
    if c {
        let r = &mut x;
        *r = v;
    }
    x
}
