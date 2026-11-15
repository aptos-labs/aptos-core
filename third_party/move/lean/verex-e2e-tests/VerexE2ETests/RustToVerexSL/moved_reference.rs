// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Holder<'a> {
    pub r: &'a mut u32,
}

pub fn into_tuple(mut a: u32, mut b: u32) -> u32 {
    let (p, q) = (&mut a, &mut b);
    *p = 1;
    *q = 2;
    a
}

pub fn into_field<'a>(h: &mut Holder<'a>, y: &'a mut u32) {
    h.r = y;
}

pub fn element(v: &mut [u32], k: usize) -> &mut u32 {
    &mut v[k]
}

pub fn written(v: &mut [u32], k: usize) {
    let e = &mut v[k];
    *e = 1;
}
