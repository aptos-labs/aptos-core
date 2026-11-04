// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// An ADT's bound on its lifetimes is not exposed, so a result that needs it is
// rejected.

pub struct Ordered<'a, 'b: 'a> {
    pub short: &'a u32,
    pub long: &'b u32,
}

pub fn ordered_long<'x, 'y>(o: Ordered<'x, 'y>) -> &'x u32 {
    o.long
}
