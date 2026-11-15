// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn write_first(s: &mut [u32]) {
    s[0] = 7;
}

pub fn through_unsize(mut a: [u32; 3]) -> u32 {
    write_first(&mut a);
    a[0]
}
