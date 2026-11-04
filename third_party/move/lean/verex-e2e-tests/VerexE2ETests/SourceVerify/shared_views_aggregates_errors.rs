// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// `final` of what a shared view holds has no reading in a postcondition.

pub struct Holder<'a> {
    pub r: &'a mut u8,
}

pub fn option_final(o: Option<&Holder<'_>>) -> u8 {
    match o {
        Some(h) => *h.r,
        None => 0,
    }
}

pub struct View<'b, 'a> {
    pub h: &'b Holder<'a>,
}

pub fn struct_final(v: View<'_, '_>) -> u8 {
    *v.h.r
}
