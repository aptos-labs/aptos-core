// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn len_of(s: &[u32]) -> usize {
    s.len()
}

pub fn write_first(s: &mut [u32]) {
    s[0] = 7;
}

pub fn shared_local(a: [u32; 3]) -> usize {
    len_of(&a)
}

pub fn shared_reference(a: &[u32; 3]) -> usize {
    len_of(a)
}

pub fn mutable_local(mut a: [u32; 3]) -> u32 {
    write_first(&mut a);
    a[0]
}

pub fn mutable_reference(a: &mut [u32; 3]) {
    write_first(a);
}

pub fn field_place(p: &mut ([u32; 2], u32)) {
    write_first(&mut p.0);
}
