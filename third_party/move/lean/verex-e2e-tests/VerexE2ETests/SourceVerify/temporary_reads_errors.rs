// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A temporary read more than once whose value reads through a reference, written
// before the last use of what the temporary feeds.

fn pick(x: u32, _y: u32) -> u32 {
    x
}

pub fn index_then_write(r: &mut usize, a: [u32; 4], v: usize) -> u32 {
    pick(a[*r], {
        *r = v;
        1
    })
}

pub fn index_then_write_false(r: &mut usize, a: [u32; 4], v: usize) -> u32 {
    pick(a[*r], {
        *r = v;
        1
    })
}
