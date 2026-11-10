// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Wrap<T>(pub T);

pub enum Choice<T> {
    None,
    Some(T),
}

pub fn wrapped(x: u32) -> u32 {
    let w = Wrap(&x);
    *w.0
}

pub fn wrapped_mut(mut x: u32) -> u32 {
    let w = Wrap(&mut x);
    *w.0 = 1;
    x
}

pub fn chosen(x: u32) -> u32 {
    let c = Choice::Some(&x);
    match c {
        Choice::Some(r) => *r,
        Choice::None => 0,
    }
}

pub fn optional(c: bool, x: &u32) -> Option<&u32> {
    if c { Some(x) } else { None }
}

pub fn optional_mut(o: Option<&mut u32>) {
    if let Some(r) = o {
        *r = 1;
    }
}
