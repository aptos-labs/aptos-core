// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Holder<'a> {
    pub r: &'a mut u32,
}

pub struct Shared<'a> {
    pub r: &'a u32,
}

pub struct Generic<'a, T> {
    pub r: &'a T,
}

pub struct Nested<'a> {
    pub holder: Holder<'a>,
    pub k: u32,
}

pub enum Slot<'a> {
    Empty,
    Full(&'a u32),
}

pub fn hold(mut x: u32) -> u32 {
    let h = Holder { r: &mut x };
    *h.r = 5;
    x
}

pub fn read(x: u32) -> u32 {
    let h = Shared { r: &x };
    *h.r
}

pub fn generic(x: u32) -> u32 {
    let h = Generic { r: &x };
    *h.r
}

pub fn nested(mut x: u32) -> u32 {
    let n = Nested {
        holder: Holder { r: &mut x },
        k: 4,
    };
    *n.holder.r = n.k;
    x
}

pub fn slot(x: u32) -> u32 {
    let s = Slot::Full(&x);
    match s {
        Slot::Full(r) => *r,
        Slot::Empty => 0,
    }
}
