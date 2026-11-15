// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn bump_all(values: &mut [u32]) {
    for x in values.iter_mut() {
        *x = x.wrapping_add(1);
    }
}

pub fn set_all(values: &mut [u32], value: u32) {
    for x in values.iter_mut() {
        *x = value;
    }
}
