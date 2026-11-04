// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct C {
    pub n: u32,
}

impl C {
    pub fn add(&mut self, d: u32) {
        self.n += d;
    }
}

// The reservation names the old referent, and the base local is assigned before the use.
pub fn base_assigned<'a>(mut p: &'a mut C, q: &'a mut C) {
    p.add({
        p = q;
        1
    });
}
