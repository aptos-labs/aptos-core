// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A shared view of a value holding a mutable reference holds what it refers to, and
// no loan. Each function has a false twin and an `ensures false` twin.

pub struct Holder<'a> {
    pub r: &'a mut u8,
}

pub struct VecHolder<'a> {
    pub v: &'a mut Vec<u8>,
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

impl<'a> VecHolder<'a> {
    pub fn len(&self) -> usize {
        self.v.len()
    }

    pub fn push_len(&mut self) {
        let n = self.len();
        self.v.push((n % 200) as u8);
    }
}

pub fn read_view(h: &Holder<'_>) -> u8 {
    *h.r
}

pub fn idx_len_field(h: &VecHolder<'_>, i: usize) -> bool {
    i < h.v.len()
}

pub fn idx_len_field_false(h: &VecHolder<'_>, i: usize) -> bool {
    i < h.v.len()
}

pub fn idx_len_field_vacuous(h: &VecHolder<'_>, i: usize) -> bool {
    i < h.v.len()
}

pub fn view_of_ref(a: u8) -> u8 {
    let mut x = a;
    let r = &mut x;
    let v = &r;
    let seen = **v;
    *r = seen % 100 + 1;
    x
}

pub fn view_of_ref_false(a: u8) -> u8 {
    let mut x = a;
    let r = &mut x;
    let v = &r;
    let seen = **v;
    *r = seen % 100 + 1;
    x
}

pub fn view_of_ref_vacuous(a: u8) -> u8 {
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

pub fn double_ref_false(m: &&mut u8) -> u8 {
    **m
}

pub fn double_ref_vacuous(m: &&mut u8) -> u8 {
    **m
}

pub fn view_callee(a: u8) -> u8 {
    let mut x = a;
    let h = Holder { r: &mut x };
    let seen = read_view(&h);
    *h.r = seen % 100 + 1;
    x
}

pub fn view_callee_false(a: u8) -> u8 {
    let mut x = a;
    let h = Holder { r: &mut x };
    let seen = read_view(&h);
    *h.r = seen % 100 + 1;
    x
}

pub fn view_callee_vacuous(a: u8) -> u8 {
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

pub fn two_phase_false(a: u8) -> u8 {
    let mut x = a;
    let mut h = Holder { r: &mut x };
    h.add(h.get());
    x
}

pub fn two_phase_vacuous(a: u8) -> u8 {
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

pub fn method_calls_view_false(a: u8) -> (u8, u8) {
    let mut x = a;
    let mut h = Holder { r: &mut x };
    let before = h.bump();
    (before, x)
}

pub fn method_calls_view_vacuous(a: u8) -> (u8, u8) {
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

pub fn view_misses_later_write_false(a: u8) -> u8 {
    let mut x = a;
    let h = Holder { r: &mut x };
    let seen = read_view(&h);
    *h.r = 7;
    seen
}

pub fn view_misses_later_write_vacuous(a: u8) -> u8 {
    let mut x = a;
    let h = Holder { r: &mut x };
    let seen = read_view(&h);
    *h.r = 7;
    seen
}

pub fn vec_len_holder(v: &mut Vec<u8>) -> usize {
    let h = VecHolder { v };
    h.len()
}

pub fn vec_len_holder_false(v: &mut Vec<u8>) -> usize {
    let h = VecHolder { v };
    h.len()
}

pub fn vec_len_holder_vacuous(v: &mut Vec<u8>) -> usize {
    let h = VecHolder { v };
    h.len()
}

pub fn vec_push_len(v: &mut Vec<u8>) {
    let mut h = VecHolder { v };
    h.push_len();
}

pub fn vec_push_len_false(v: &mut Vec<u8>) {
    let mut h = VecHolder { v };
    h.push_len();
}

pub fn vec_push_len_vacuous(v: &mut Vec<u8>) {
    let mut h = VecHolder { v };
    h.push_len();
}

// A `&mut ()` under a view holds a real unit.
pub struct UnitHolder<'a> {
    pub r: &'a mut (),
}

pub fn read_unit<'a>(h: &'a UnitHolder<'_>) -> &'a () {
    &*h.r
}

pub fn unit_under_view(a: u8) -> u8 {
    let mut u = ();
    let h = UnitHolder { r: &mut u };
    read_unit(&h);
    *h.r = ();
    a
}

pub fn unit_under_view_false(a: u8) -> u8 {
    let mut u = ();
    let h = UnitHolder { r: &mut u };
    read_unit(&h);
    *h.r = ();
    a
}

pub fn unit_under_view_vacuous(a: u8) -> u8 {
    let mut u = ();
    let h = UnitHolder { r: &mut u };
    read_unit(&h);
    *h.r = ();
    a
}

// A generic `T` under a view, at `()`.
pub fn peek<'a, T>(m: &'a &mut T) -> &'a T {
    &**m
}

pub fn generic_at_unit(a: u8) -> u8 {
    let mut u = ();
    let r = &mut u;
    let v = &r;
    peek(v);
    a
}

pub fn generic_at_unit_false(a: u8) -> u8 {
    let mut u = ();
    let r = &mut u;
    let v = &r;
    peek(v);
    a
}

pub fn generic_at_unit_vacuous(a: u8) -> u8 {
    let mut u = ();
    let r = &mut u;
    let v = &r;
    peek(v);
    a
}

// `&T` at `T := &mut u8`, compiled at the argument.
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

pub fn generic_at_reference_false(a: u8) -> u8 {
    let mut x = a;
    let r = &mut x;
    let seen = **same(&r);
    *r = seen % 100 + 1;
    x
}

pub fn generic_at_reference_vacuous(a: u8) -> u8 {
    let mut x = a;
    let r = &mut x;
    let seen = **same(&r);
    *r = seen % 100 + 1;
    x
}
