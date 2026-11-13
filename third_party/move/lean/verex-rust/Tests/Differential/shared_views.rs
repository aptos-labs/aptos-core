// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A shared view of a value holding a mutable reference holds what it refers to, and
// no loan.

pub struct Holder<'a> {
    pub r: &'a mut u8,
}

impl<'a> Holder<'a> {
    pub fn get(&self) -> u8 {
        *self.r
    }

    pub fn add(&mut self, d: u8) {
        *self.r = *self.r % 100 + d % 100;
    }

    pub fn bump(&mut self) -> u8 {
        let before = self.get();
        *self.r = before % 100 + 1;
        before
    }
}

pub fn read_view(h: &Holder<'_>) -> u8 {
    *h.r
}

pub fn view_of_ref(a: u8) -> u8 {
    let mut x = a;
    let r = &mut x;
    let v = &r;
    let seen = **v;
    *r = seen % 100 + 1;
    x
}

pub fn double_ref(m: &&mut u8) -> u8 {
    **m
}

pub fn view_callee(a: u8) -> u8 {
    let mut x = a;
    let h = Holder { r: &mut x };
    let seen = read_view(&h);
    *h.r = seen % 100 + 1;
    x
}

pub fn two_phase(a: u8) -> u8 {
    let mut x = a;
    let mut h = Holder { r: &mut x };
    h.add(h.get());
    x
}

pub fn method_calls_view(a: u8) -> (u8, u8) {
    let mut x = a;
    let mut h = Holder { r: &mut x };
    let before = h.bump();
    (before, x)
}

pub fn view_misses_later_write(a: u8) -> u8 {
    let mut x = a;
    let h = Holder { r: &mut x };
    let seen = read_view(&h);
    *h.r = 7;
    seen
}

// A `&mut ()` under a view holds a real unit.
pub struct UnitHolder<'a> {
    pub r: &'a mut (),
}

pub fn read_unit<'a>(h: &'a UnitHolder<'_>) -> &'a () {
    &*h.r
}

pub fn unit_under_view(u: &mut (), a: u8) -> u8 {
    let h = UnitHolder { r: u };
    read_unit(&h);
    a
}

pub fn same<T>(a: &T) -> &T {
    a
}

pub fn generic_at_reference(a: u8) -> u8 {
    let mut x = a;
    let r = &mut x;
    let seen = **same(&r);
    *r = seen % 100 + 1;
    x
}
