// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// What a specification may call: a body reading through a shared reference, a
// body rebinding its own locals, and one taking `&mut` only to read through it.
use std::ops::RangeInclusive;

pub fn low(range: &RangeInclusive<u32>) -> u32 {
    *range.start()
}

pub fn incremented(a: u32) -> u32 {
    let mut i = a;
    i = i + 1;
    i
}

pub fn read_through(place: &mut u32) -> u32 {
    *place
}

pub fn uses_low(_range: &RangeInclusive<u32>) -> bool {
    true
}

pub fn uses_rebinding(a: u32) -> u32 {
    a + 1
}

pub fn uses_read(place: &mut u32) -> u32 {
    *place
}
