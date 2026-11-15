// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![feature(core_intrinsics)]
#![allow(internal_features)]

pub fn tuple_place(a: (u8, i8), b: u8) -> (u8, i8) {
    let mut t = a;
    t.0 = b;
    t.1 = core::intrinsics::wrapping_sub(0, t.1);
    t
}

pub fn borrow_field(a: u8, b: i8) -> (u8, i8) {
    let mut t = (a, b);
    let r = &mut t.0;
    *r = core::intrinsics::wrapping_add(*r, 1);
    t
}

pub fn nested_tuple(a: ((u8, u8), u8)) -> u8 {
    let mut t = a;
    (t.0).1 = t.1;
    let r = &mut (t.0).0;
    *r = core::intrinsics::wrapping_add(*r, 1);
    (t.0).0 ^ (t.0).1
}

pub fn write_field(a: &mut (u8, u8), b: u8) {
    a.1 = b;
}

pub fn tuple_move(a: (u8, (bool, i8))) -> (i8, bool, u8) {
    let (x, (y, z)) = a;
    (z, y, x)
}

pub fn element_field(a: [(u8, i8); 2], i: usize) -> u8 {
    a[i].0
}
