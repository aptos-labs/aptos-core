// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// An input ADT's declared lifetime bound also holds in this function.

pub struct Ordered<'a, 'b: 'a> {
    pub short: &'a u32,
    pub long: &'b u32,
}

pub fn ordered_long<'x, 'y>(o: Ordered<'x, 'y>) -> &'x u32 {
    o.long
}
