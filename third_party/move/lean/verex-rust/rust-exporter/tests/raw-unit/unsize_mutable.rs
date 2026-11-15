// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn mutable_local(mut a: [u32; 3]) {
    let s: &mut [u32] = &mut a;
    s[0] = 7;
}

pub fn mutable_reference(a: &mut [u32; 3]) {
    let s: &mut [u32] = a;
    s[0] = 7;
}

pub fn field_place(p: &mut ([u32; 2], u32)) {
    let s: &mut [u32] = &mut p.0;
    s[0] = 7;
}
