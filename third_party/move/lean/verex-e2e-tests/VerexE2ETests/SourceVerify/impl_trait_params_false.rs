// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// An `impl Trait` parameter is a generic parameter: the body sees no more of it than of
// any type parameter, and a call states it by what it passes.

pub fn keep(a: u64, _tag: impl Copy) -> u64 {
    a
}

pub fn first_of(pair: &(u64, impl Copy)) -> u64 {
    pair.0
}

pub fn between(_a: impl Copy, b: u64, _c: impl Copy) -> u64 {
    b
}

pub fn named_beside<Impl1>(x: Impl1, _tag: impl Copy) -> Impl1 {
    x
}

pub fn at_two_types(a: u64) -> u64 {
    keep(a, 3u8) / 2 + first_of(&(a, true)) / 2
}
