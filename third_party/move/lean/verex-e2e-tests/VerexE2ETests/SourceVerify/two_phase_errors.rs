// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A two-phase borrow activates at its use: reads between see the old value. Each
// function has a false twin, whose clause is reported here.

pub struct C {
    pub n: u32,
}

impl C {
    pub fn get(&self) -> u32 {
        self.n
    }

    pub fn add(&mut self, d: u32) {
        self.n = self.n + d;
    }
}

pub struct Holder<'a> {
    pub r: &'a mut u32,
}

pub fn add_to(target: &mut u32, d: u32) {
    *target = *target + d;
}

pub fn two_phase_local(n: u32) -> u32 {
    let mut c = C { n };
    c.add(c.get());
    c.n
}

pub fn two_phase_local_false(n: u32) -> u32 {
    let mut c = C { n };
    c.add(c.get());
    c.n
}

pub fn two_phase_free(c: &mut C) {
    c.add(c.get());
}

pub fn two_phase_free_false(c: &mut C) {
    c.add(c.get());
}

pub fn push_len(v: &mut Vec<usize>) {
    v.push(v.len());
}

pub fn push_len_false(v: &mut Vec<usize>) {
    v.push(v.len());
}

pub fn holder(h: Holder<'_>) {
    add_to(h.r, *h.r);
}

pub fn holder_false(h: Holder<'_>) {
    add_to(h.r, *h.r);
}

pub fn index_self(v: &mut Vec<u32>) {
    v.push(v[0]);
}

pub fn index_self_false(v: &mut Vec<u32>) {
    v.push(v[0]);
}

pub fn index_operand(c: &mut C, a: &[u32], i: usize) {
    c.add(a[i]);
}

pub fn index_operand_false(c: &mut C, a: &[u32], i: usize) {
    c.add(a[i]);
}

// The use is in another block than the reservation.
pub fn across_blocks(b: bool, n: u32) -> u32 {
    let mut c = C { n };
    c.add(if b { c.get() } else { 1 });
    c.n
}

pub fn across_blocks_false(b: bool, n: u32) -> u32 {
    let mut c = C { n };
    c.add(if b { c.get() } else { 1 });
    c.n
}
