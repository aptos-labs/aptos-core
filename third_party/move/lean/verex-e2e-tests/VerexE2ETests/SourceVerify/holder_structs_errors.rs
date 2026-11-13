// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct R {
    pub x: u32,
    pub y: u32,
}

pub struct Holder<'a> {
    pub r: &'a mut u32,
}

pub struct Two<'a> {
    pub a: &'a mut u32,
    pub b: &'a mut u32,
}

pub fn hold_write_read(mut x: u32) -> u32 {
    let h = Holder { r: &mut x };
    *h.r = 5;
    x
}

pub fn hold_write_read_false(mut x: u32) -> u32 {
    let h = Holder { r: &mut x };
    *h.r = 5;
    x
}

pub fn hold_param(x: &mut u32) {
    let h = Holder { r: x };
    *h.r = 3;
}

pub fn hold_param_false(x: &mut u32) {
    let h = Holder { r: x };
    *h.r = 3;
}

pub fn hold_move_out(mut x: u32) -> u32 {
    let h = Holder { r: &mut x };
    let r = h.r;
    *r = 5;
    x
}

pub fn hold_move_out_false(mut x: u32) -> u32 {
    let h = Holder { r: &mut x };
    let r = h.r;
    *r = 5;
    x
}

pub fn store(h: Holder<'_>) {
    *h.r = 7;
}

pub fn hold_callee_val(mut x: u32) -> u32 {
    store(Holder { r: &mut x });
    x
}

pub fn hold_callee_val_false(mut x: u32) -> u32 {
    store(Holder { r: &mut x });
    x
}

pub fn store_mut(h: &mut Holder<'_>) {
    *h.r = 7;
}

pub fn hold_callee_mut(mut x: u32) -> u32 {
    let mut h = Holder { r: &mut x };
    store_mut(&mut h);
    *h.r = *h.r + 1;
    x
}

pub fn hold_callee_mut_false(mut x: u32) -> u32 {
    let mut h = Holder { r: &mut x };
    store_mut(&mut h);
    *h.r = *h.r + 1;
    x
}

pub fn mk<'a>(x: &'a mut u32) -> Holder<'a> {
    Holder { r: x }
}

pub fn hold_return(mut x: u32) -> u32 {
    let h = mk(&mut x);
    *h.r = 9;
    x
}

pub fn hold_return_false(mut x: u32) -> u32 {
    let h = mk(&mut x);
    *h.r = 9;
    x
}

pub fn hold_move_local(mut x: u32) -> u32 {
    let h = Holder { r: &mut x };
    store(h);
    x
}

pub fn hold_move_local_false(mut x: u32) -> u32 {
    let h = Holder { r: &mut x };
    store(h);
    x
}

pub fn hold_reassign(mut a: u32, mut b: u32) -> R {
    let mut h = Holder { r: &mut a };
    *h.r = 1;
    h.r = &mut b;
    *h.r = 2;
    R { x: a, y: b }
}

pub fn hold_reassign_false(mut a: u32, mut b: u32) -> R {
    let mut h = Holder { r: &mut a };
    *h.r = 1;
    h.r = &mut b;
    *h.r = 2;
    R { x: a, y: b }
}

pub fn hold_join(c: bool, mut a: u32, mut b: u32) -> R {
    let h = if c { Holder { r: &mut a } } else { Holder { r: &mut b } };
    *h.r = 1;
    R { x: a, y: b }
}

pub fn hold_join_false(c: bool, mut a: u32, mut b: u32) -> R {
    let h = if c { Holder { r: &mut a } } else { Holder { r: &mut b } };
    *h.r = 1;
    R { x: a, y: b }
}

pub fn two_local(mut a: u32, mut b: u32) -> R {
    let t = Two { a: &mut a, b: &mut b };
    *t.a = 1;
    *t.b = 2;
    R { x: a, y: b }
}

pub fn two_local_false(mut a: u32, mut b: u32) -> R {
    let t = Two { a: &mut a, b: &mut b };
    *t.a = 1;
    *t.b = 2;
    R { x: a, y: b }
}
