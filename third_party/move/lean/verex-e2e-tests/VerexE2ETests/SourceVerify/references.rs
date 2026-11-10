// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Pair {
    pub first: u32,
    pub second: u32,
}

pub fn swap(left: &mut u32, right: &mut u32) {
    let held = *left;
    *left = *right;
    *right = held;
}

pub fn swap_fields(pair: &mut Pair) {
    swap(&mut pair.first, &mut pair.second);
}

pub fn clear(pair: &mut Pair) {
    pair.first = 0;
}

pub fn clear_twice(pair: &mut Pair) {
    clear(pair);
    clear(pair);
}
