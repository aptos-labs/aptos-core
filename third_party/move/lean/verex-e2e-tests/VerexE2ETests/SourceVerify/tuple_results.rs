// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn swapped(a: u32, b: u32) -> (u32, u32) {
    (b, a)
}

pub fn same(t: (u32, u32)) -> (u32, u32) {
    t
}

pub fn first(t: (u32, u32)) -> u32 {
    t.0
}

pub fn nested(t: ((u32, u32), u32)) -> u32 {
    (t.0).1
}

pub fn written(t: &mut (u32, u32)) {
    t.0 = 1;
}
