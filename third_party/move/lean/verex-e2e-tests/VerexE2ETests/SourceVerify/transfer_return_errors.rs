// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A return between a reference's move out of a holder and the holder's overwrite: no
// transfer, so the move reads as a reborrow through the holder. Each function has a false
// twin and an `ensures false` twin.

pub struct Holder<'a> {
    pub r: &'a mut u8,
}

pub fn step(r: &mut u8) -> &mut u8 {
    r
}

pub fn return_between(c: bool, a: &mut u8) {
    let mut h = Holder { r: a };
    let r = h.r;
    *r = 1;
    if c {
        return;
    }
    h = Holder { r: step(r) };
    *h.r = 2;
}

pub fn return_between_false(c: bool, a: &mut u8) {
    let mut h = Holder { r: a };
    let r = h.r;
    *r = 1;
    if c {
        return;
    }
    h = Holder { r: step(r) };
    *h.r = 2;
}

pub fn return_between_vacuous(c: bool, a: &mut u8) {
    let mut h = Holder { r: a };
    let r = h.r;
    *r = 1;
    if c {
        return;
    }
    h = Holder { r: step(r) };
    *h.r = 2;
}
