// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// `drop` is the drop of its argument, as the end of a scope is.

pub struct Holder<'a> {
    pub r: &'a mut u8,
}

pub fn dropped_holder(a: u8) -> u8 {
    let mut x = a;
    let h = Holder { r: &mut x };
    *h.r = a % 100 + 1;
    drop(h);
    x
}

pub fn dropped_reference(a: u8) -> u8 {
    let mut x = a;
    let r = &mut x;
    *r = a % 100 + 2;
    drop(r);
    x
}

pub fn dropped_value(a: u8) -> u8 {
    drop(a);
    a
}
