// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![feature(core_intrinsics)]
#![allow(internal_features)]

pub enum Slot<'a> {
    Left(&'a mut u8),
    Right(&'a mut u8),
    Empty,
}

pub fn slot_write(s: Slot<'_>, v: u8) {
    match s {
        Slot::Left(r) => *r = v,
        Slot::Right(r) => *r = core::intrinsics::wrapping_add(v, 1),
        Slot::Empty => {}
    }
}

pub fn slot_repointed(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let mut s = Slot::Left(&mut x);
    if c {
        s = Slot::Right(&mut y);
    }
    slot_write(s, v);
    (x, y)
}

pub fn slot_repointed_false(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let mut s = Slot::Left(&mut x);
    if c {
        s = Slot::Right(&mut y);
    }
    slot_write(s, v);
    (x, y)
}

pub fn slot_repointed_vacuous(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let mut s = Slot::Left(&mut x);
    if c {
        s = Slot::Right(&mut y);
    }
    slot_write(s, v);
    (x, y)
}
