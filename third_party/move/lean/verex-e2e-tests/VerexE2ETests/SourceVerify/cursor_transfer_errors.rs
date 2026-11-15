// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A reference moved out of a holder an overwrite follows is transferred: the holder's
// component is cleared and the moved reference is the loan's only holder. Each function
// has a false twin and an `ensures false` twin.

pub struct Holder<'a> {
    pub r: &'a mut u8,
}

pub struct Tagged<'a> {
    pub r: &'a mut u8,
    pub n: u8,
}

pub fn step(r: &mut u8) -> &mut u8 {
    r
}

pub fn transfer_write(a: &mut u8) {
    let mut h = Holder { r: a };
    let r = h.r;
    *r = 7;
    h = Holder { r: step(r) };
    let _ = h;
}

pub fn transfer_write_false(a: &mut u8) {
    let mut h = Holder { r: a };
    let r = h.r;
    *r = 7;
    h = Holder { r: step(r) };
    let _ = h;
}

pub fn transfer_write_vacuous(a: &mut u8) {
    let mut h = Holder { r: a };
    let r = h.r;
    *r = 7;
    h = Holder { r: step(r) };
    let _ = h;
}

pub fn write_both(a: &mut u8) {
    let mut h = Holder { r: a };
    let r = h.r;
    *r = 3;
    h = Holder { r: step(r) };
    *h.r += 4;
}

pub fn write_both_false(a: &mut u8) {
    let mut h = Holder { r: a };
    let r = h.r;
    *r = 3;
    h = Holder { r: step(r) };
    *h.r += 4;
}

pub fn write_both_vacuous(a: &mut u8) {
    let mut h = Holder { r: a };
    let r = h.r;
    *r = 3;
    h = Holder { r: step(r) };
    *h.r += 4;
}

pub fn step_direct(a: &mut u8) {
    let mut h = Holder { r: a };
    h = Holder { r: step(h.r) };
    *h.r = 5;
}

pub fn step_direct_false(a: &mut u8) {
    let mut h = Holder { r: a };
    h = Holder { r: step(h.r) };
    *h.r = 5;
}

pub fn step_direct_vacuous(a: &mut u8) {
    let mut h = Holder { r: a };
    h = Holder { r: step(h.r) };
    *h.r = 5;
}

pub fn other_field_between(a: &mut u8, k: u8) -> u8 {
    let mut t = Tagged { r: a, n: k };
    let r = t.r;
    let seen = t.n;
    t = Tagged { r: step(r), n: 0 };
    *t.r = seen;
    t.n
}

pub fn other_field_between_false(a: &mut u8, k: u8) -> u8 {
    let mut t = Tagged { r: a, n: k };
    let r = t.r;
    let seen = t.n;
    t = Tagged { r: step(r), n: 0 };
    *t.r = seen;
    t.n
}

pub fn other_field_between_vacuous(a: &mut u8, k: u8) -> u8 {
    let mut t = Tagged { r: a, n: k };
    let r = t.r;
    let seen = t.n;
    t = Tagged { r: step(r), n: 0 };
    *t.r = seen;
    t.n
}
