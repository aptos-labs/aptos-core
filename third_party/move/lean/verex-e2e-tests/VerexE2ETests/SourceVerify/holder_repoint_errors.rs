// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Holder<'a> {
    pub r: &'a mut u32,
}

pub fn repoint<'a>(h: Holder<'a>, y: &'a mut u32) -> Holder<'a> {
    let mut k = h;
    k.r = y;
    k
}

pub fn caller(mut a: u32, mut b: u32) -> (u32, u32) {
    let k = repoint(Holder { r: &mut a }, &mut b);
    *k.r = 9;
    (a, b)
}
