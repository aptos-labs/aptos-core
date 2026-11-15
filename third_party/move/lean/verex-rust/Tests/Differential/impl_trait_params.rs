// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// An `impl Trait` parameter runs as any type parameter, at the type the input has.

pub fn keep(a: u8, _tag: impl Copy) -> u8 {
    a
}

pub fn first_of(pair: &(u8, impl Copy)) -> u8 {
    pair.0
}

pub fn between(_a: impl Copy, b: u8, _c: impl Copy) -> u8 {
    b
}

pub fn same(x: impl Copy) -> bool {
    let _y = x;
    true
}
