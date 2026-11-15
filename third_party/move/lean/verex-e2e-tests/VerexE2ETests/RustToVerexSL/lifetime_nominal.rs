// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Shared<'a> {
    pub r: &'a u32,
}

pub fn make<'a>(x: &'a u32) -> Shared<'a> {
    Shared { r: x }
}

pub fn value(h: Shared<'_>) -> u32 {
    *h.r
}

pub fn through(x: u32) -> u32 {
    let h = make(&x);
    value(h)
}

pub fn joined(c: bool, a: u32, b: u32) -> u32 {
    let h = if c { Shared { r: &a } } else { Shared { r: &b } };
    *h.r
}

pub fn larger<'a>(a: Shared<'a>, b: Shared<'a>) -> Shared<'a> {
    if *a.r > *b.r { a } else { b }
}

pub fn replace<'a>(h: &mut Shared<'a>, y: &'a u32) {
    h.r = y;
}
