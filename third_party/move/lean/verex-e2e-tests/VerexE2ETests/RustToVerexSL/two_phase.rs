// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct C {
    pub n: u32,
}

impl C {
    pub fn get(&self) -> u32 {
        self.n
    }

    pub fn add(&mut self, d: u32) {
        self.n += d;
    }
}

pub struct Pair {
    pub a: C,
    pub b: C,
}

pub struct Holder<'a> {
    pub r: &'a mut u32,
}

pub fn add_to(target: &mut u32, d: u32) {
    *target += d;
}

pub fn two_phase_local(n: u32) -> u32 {
    let mut c = C { n };
    c.add(c.get());
    c.n
}

pub fn two_phase_free(c: &mut C) {
    c.add(c.get());
}

pub fn push_len(v: &mut Vec<usize>) {
    v.push(v.len());
}

pub fn holder(h: Holder<'_>) {
    add_to(h.r, *h.r);
}

pub fn index_operand(c: &mut C, a: &[u32], i: usize) {
    c.add(a[i]);
}

pub fn index_self(v: &mut Vec<u32>) {
    v.push(v[0]);
}

pub fn local_across(b: bool, n: u32) -> u32 {
    let mut c = C { n };
    c.add(if b { c.get() } else { 1 });
    c.n
}

pub fn disjoint_fields(mut p: Pair) -> u32 {
    p.a.add(p.b.get());
    p.b.add(1);
    p.a.add({
        p.b.add(1);
        p.b.get()
    });
    p.a.n
}
