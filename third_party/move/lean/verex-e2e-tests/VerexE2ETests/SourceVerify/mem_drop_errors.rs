// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// `drop` of a value with a destructor has no denotation, as the end of a scope has none.

pub struct Guard(pub u32);

impl Drop for Guard {
    fn drop(&mut self) {}
}

pub struct Holder<'a> {
    pub r: &'a mut u8,
}

impl Drop for Holder<'_> {
    fn drop(&mut self) {
        *self.r = 9;
    }
}

pub struct Plain<'a> {
    pub r: &'a mut u8,
}

pub fn value(g: Guard) -> u32 {
    drop(g);
    1
}

pub fn boxed(g: Box<Guard>) -> u32 {
    drop(g);
    1
}

pub fn holder(a: u8) -> u8 {
    let mut x = a;
    let h = Holder { r: &mut x };
    drop(h);
    x
}

// A generic theorem does not transfer to a type holding `&mut`, and the drop of a
// parameter there is not read at the rows.
pub fn generic<T>(x: T) {
    drop(x);
}

pub fn generic_at_guard(g: Guard) -> u32 {
    generic(g);
    1
}

pub fn generic_at_holder(a: u8) -> u8 {
    let mut x = a;
    let h = Plain { r: &mut x };
    generic(h);
    x
}

pub fn scope_end<T>(_x: T) {}

pub fn scope_end_at_guard(g: Guard) -> u32 {
    scope_end(g);
    1
}
