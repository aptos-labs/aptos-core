// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A call in a specification is read as a value, which has no reading of an effect
// on a reference: `two` advances the iterator, and read as a value would be `one`.
pub fn one(a: u32, b: u32) -> bool {
    let mut it = a..=b;
    it.next().is_some()
}

pub fn two(a: u32, b: u32) -> bool {
    let mut it = a..=b;
    let _ = it.next();
    it.next().is_some()
}

pub fn check(a: u32, b: u32) -> bool {
    a == b
}
