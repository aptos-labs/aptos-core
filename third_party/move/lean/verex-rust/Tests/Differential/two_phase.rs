// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct C {
    pub n: u8,
}

impl C {
    pub fn get(&self) -> u8 {
        self.n
    }

    pub fn add(&mut self, d: u8) {
        self.n += d;
    }
}

pub struct Holder<'a> {
    pub r: &'a mut u8,
}

pub fn add_to(target: &mut u8, d: u8) {
    *target += d;
}

pub fn two_phase_local(n: u8) -> u8 {
    let mut c = C { n };
    c.add(c.get());
    c.n
}

pub fn two_phase_free(c: &mut C) {
    c.add(c.get());
}

pub fn holder(h: Holder<'_>) {
    add_to(h.r, *h.r);
}

pub fn across_blocks(b: bool, n: u8) -> u8 {
    let mut c = C { n };
    c.add(if b { c.get() } else { 1 });
    c.n
}
