// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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
