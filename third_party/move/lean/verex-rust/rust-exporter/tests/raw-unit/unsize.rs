// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn len_of(s: &[u32]) -> usize {
    s.len()
}

pub fn shared_local(a: [u32; 3]) -> usize {
    len_of(&a)
}

pub fn shared_reference(a: &[u32; 3]) -> usize {
    len_of(a)
}
