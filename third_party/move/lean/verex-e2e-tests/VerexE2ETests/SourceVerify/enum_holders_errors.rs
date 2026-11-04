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

pub fn opt_pick(c: bool, a: &mut u8) -> Option<&mut u8> {
    if c { Some(a) } else { None }
}

pub fn opt_by_value(o: Option<&mut u8>, v: u8) {
    if let Some(r) = o {
        *r = v;
    }
}

pub fn opt_by_value_false(o: Option<&mut u8>, v: u8) {
    if let Some(r) = o {
        *r = v;
    }
}

pub fn opt_by_value_vacuous(o: Option<&mut u8>, v: u8) {
    if let Some(r) = o {
        *r = v;
    }
}

pub fn opt_by_value_caller(c: bool, a: u8, v: u8) -> u8 {
    let mut x = a;
    let o = if c { Some(&mut x) } else { None };
    if let Some(r) = o {
        *r = v;
    }
    x
}

pub fn opt_by_value_caller_false(c: bool, a: u8, v: u8) -> u8 {
    let mut x = a;
    let o = if c { Some(&mut x) } else { None };
    if let Some(r) = o {
        *r = v;
    }
    x
}

pub fn opt_by_value_caller_vacuous(c: bool, a: u8, v: u8) -> u8 {
    let mut x = a;
    let o = if c { Some(&mut x) } else { None };
    if let Some(r) = o {
        *r = v;
    }
    x
}

pub fn opt_returned(c: bool, a: &mut u8, v: u8) {
    if let Some(r) = opt_pick(c, a) {
        *r = v;
    }
}

pub fn opt_returned_false(c: bool, a: &mut u8, v: u8) {
    if let Some(r) = opt_pick(c, a) {
        *r = v;
    }
}

pub fn opt_returned_vacuous(c: bool, a: &mut u8, v: u8) {
    if let Some(r) = opt_pick(c, a) {
        *r = v;
    }
}

pub fn opt_by_ref(o: &mut Option<u8>, v: u8) {
    if let Some(r) = o {
        *r = v;
    }
}

pub fn opt_by_ref_false(o: &mut Option<u8>, v: u8) {
    if let Some(r) = o {
        *r = v;
    }
}

pub fn opt_by_ref_vacuous(o: &mut Option<u8>, v: u8) {
    if let Some(r) = o {
        *r = v;
    }
}

pub fn opt_ref_of_ref(o: &mut Option<&mut u8>, v: u8) {
    if let Some(r) = o {
        **r = v;
    }
}

pub fn opt_ref_of_ref_false(o: &mut Option<&mut u8>, v: u8) {
    if let Some(r) = o {
        **r = v;
    }
}

pub fn opt_ref_of_ref_vacuous(o: &mut Option<&mut u8>, v: u8) {
    if let Some(r) = o {
        **r = v;
    }
}

pub fn opt_none_on_one_path(c: bool, a: u8, v: u8) -> u8 {
    let mut x = a;
    let mut o = Some(&mut x);
    if c {
        o = None;
    }
    if let Some(r) = o {
        *r = v;
    }
    x
}

pub fn opt_none_on_one_path_false(c: bool, a: u8, v: u8) -> u8 {
    let mut x = a;
    let mut o = Some(&mut x);
    if c {
        o = None;
    }
    if let Some(r) = o {
        *r = v;
    }
    x
}

pub fn opt_none_on_one_path_vacuous(c: bool, a: u8, v: u8) -> u8 {
    let mut x = a;
    let mut o = Some(&mut x);
    if c {
        o = None;
    }
    if let Some(r) = o {
        *r = v;
    }
    x
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
