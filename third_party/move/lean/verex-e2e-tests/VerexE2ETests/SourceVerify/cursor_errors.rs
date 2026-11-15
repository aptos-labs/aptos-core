// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A reference moved out of a holder an overwrite follows is transferred: the holder's
// component is cleared and the moved reference is the loan's only holder. Each function
// has a false twin and an `ensures false` twin.

pub struct Holder<'a> {
    pub r: &'a mut u8,
}

pub fn step(r: &mut u8) -> &mut u8 {
    r
}

pub fn cursor(a: &mut u8) {
    let mut h = Holder { r: a };
    while *h.r != 0 {
        *h.r -= 1;
        let r = h.r;
        h = Holder { r: step(r) };
    }
}

pub fn cursor_false(a: &mut u8) {
    let mut h = Holder { r: a };
    while *h.r != 0 {
        *h.r -= 1;
        let r = h.r;
        h = Holder { r: step(r) };
    }
}

pub fn cursor_vacuous(a: &mut u8) {
    let mut h = Holder { r: a };
    while *h.r != 0 {
        *h.r -= 1;
        let r = h.r;
        h = Holder { r: step(r) };
    }
}

pub fn local_cursor(a: u8) -> u8 {
    let mut x = a;
    let mut h = Holder { r: &mut x };
    while *h.r != 0 {
        *h.r -= 1;
        let r = h.r;
        h = Holder { r: step(r) };
    }
    x
}

pub fn local_cursor_false(a: u8) -> u8 {
    let mut x = a;
    let mut h = Holder { r: &mut x };
    while *h.r != 0 {
        *h.r -= 1;
        let r = h.r;
        h = Holder { r: step(r) };
    }
    x
}

pub fn local_cursor_vacuous(a: u8) -> u8 {
    let mut x = a;
    let mut h = Holder { r: &mut x };
    while *h.r != 0 {
        *h.r -= 1;
        let r = h.r;
        h = Holder { r: step(r) };
    }
    x
}

// `final` of what a shared view holds, and of a place holding no reference.
pub fn final_through_view(h: &Holder<'_>, n: u8) -> u8 {
    let mut i = 0u8;
    while i < n {
        i += 1;
    }
    i + *h.r
}
