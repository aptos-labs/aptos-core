// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Signatures keep their declared lifetimes, and their bodies stay inferred. An
// elided lifetime is named only where it tells a result from a parameter.

pub struct Holder<'a> {
    pub r: &'a mut u32,
}

pub fn sel<'a, 'b>(x: &'a mut u32, _y: &'b mut u32) -> &'a mut u32 {
    let r = &mut *x;
    r
}

pub fn named_unit<'a>(x: &'a mut u32) {
    *x = 1;
}

pub fn elided_unit(x: &mut u32, y: &u32) {
    *x = *y;
}

pub fn elided_one(x: &mut u32) -> &mut u32 {
    x
}

pub fn elided_with_named<'a>(x: &'a mut u32, _y: &mut u32) -> &'a mut u32 {
    x
}

// The bridge keeps early-bound lifetimes and their explicit outlives bound.
pub fn outlives<'s, 'l: 's>(x: &'l mut u32) -> &'s mut u32 {
    x
}

pub fn outlives_static<'a: 'static>(x: &'a u32) -> &'static u32 {
    x
}

pub fn generic<'a, T>(x: &'a mut T, _y: &u32) -> &'a mut T {
    x
}

pub fn static_result<'a>(_x: &'a u32, y: &'static u32) -> &'static u32 {
    y
}

pub fn holder<'a>(x: &'a mut u32) -> Holder<'a> {
    Holder { r: x }
}

pub fn inner<'s, 'a>(h: &'s mut Holder<'a>) -> &'s mut u32 {
    &mut *h.r
}

pub trait Convert<'a, 'b> {
    fn convert(self, x: &'a mut u32) -> &'b mut u32;
}

// Trait-bound early lifetimes remain distinct in the declared signature.
pub fn bounded<'a, 'b, T: Convert<'a, 'b>>(_value: &T, _x: &'a mut u32, y: &'b mut u32) -> &'b mut u32 {
    y
}

pub struct S {
    pub v: u32,
}

impl S {
    pub fn first(&mut self, _other: &u32) -> &mut u32 {
        &mut self.v
    }
}

impl<'a> Holder<'a> {
    pub fn get(&mut self) -> &mut u32 {
        &mut *self.r
    }

    pub fn take(self) -> &'a mut u32 {
        self.r
    }
}

pub fn caller(mut a: u32, mut b: u32) -> u32 {
    let r = sel(&mut a, &mut b);
    b = 2;
    *r = 1;
    a + b
}

pub fn vec_index(v: &mut Vec<u32>, i: usize) -> &mut u32 {
    &mut v[i]
}
