// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn len_of(s: &[u32]) -> usize {
    s.len()
}

pub fn first_or_zero(s: &[u32]) -> u32 {
    if s.len() == 0 { 0 } else { s[0] }
}

pub fn array_length(a: [u32; 3]) -> usize {
    len_of(&a)
}

pub fn array_first(a: [u32; 2]) -> u32 {
    first_or_zero(&a)
}

pub fn borrowed_array_first(a: &[u32; 2]) -> u32 {
    first_or_zero(a)
}
