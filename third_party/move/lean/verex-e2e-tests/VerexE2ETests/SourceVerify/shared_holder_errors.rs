// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct SHolder<'a> {
    pub r: &'a u32,
}

pub enum Slot<'a> {
    Empty,
    Full(&'a u32),
}

pub fn read(x: u32) -> u32 {
    let h = SHolder { r: &x };
    *h.r
}

pub fn get(h: SHolder<'_>) -> u32 {
    *h.r
}

pub fn make<'a>(x: &'a u32) -> SHolder<'a> {
    SHolder { r: x }
}

pub fn through(x: u32) -> u32 {
    let h = make(&x);
    get(h)
}

pub fn lender_written(mut x: u32) -> u32 {
    x = 1;
    let h = SHolder { r: &x };
    let a = *h.r;
    x = 2;
    a + x
}

pub fn slot(x: u32) -> u32 {
    let s = Slot::Full(&x);
    match s {
        Slot::Full(r) => *r,
        Slot::Empty => 0,
    }
}
