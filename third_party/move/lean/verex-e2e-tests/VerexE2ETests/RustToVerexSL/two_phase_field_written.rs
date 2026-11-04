// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Holder<'a> {
    pub r: &'a mut u32,
}

pub fn add_to(target: &mut u32, d: u32) {
    *target += d;
}

// The holder's field the reservation goes through is written before the use.
pub fn field_written<'a>(h: &mut Holder<'a>, q: &'a mut u32) {
    add_to(h.r, {
        h.r = q;
        1
    });
}
