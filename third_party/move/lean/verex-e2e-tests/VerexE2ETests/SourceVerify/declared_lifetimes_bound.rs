// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The exporter's predicate queries preserve the ADT's outlives bound,
// so a reference with the longer lifetime can be returned at the shorter one.

pub struct Ordered<'a, 'b: 'a> {
    pub short: &'a u32,
    pub long: &'b u32,
}

pub fn ordered_long<'x, 'y>(o: Ordered<'x, 'y>) -> &'x u32 {
    o.long
}
