// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A proof step is a specification expression like a clause.
pub fn advance(place: &mut u32) -> u32 {
    *place = 1;
    *place
}

pub fn check(a: u32) -> u32 {
    a
}
