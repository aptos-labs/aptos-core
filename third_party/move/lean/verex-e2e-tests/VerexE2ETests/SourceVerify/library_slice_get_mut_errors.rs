// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn preserved(values: &mut [u32], i: usize) {
    if let Some(x) = values.get_mut(i) {
        *x = x.wrapping_add(1);
    }
}

pub fn increment(values: &mut [u32], i: usize) {
    if let Some(x) = values.get_mut(i) {
        *x = x.wrapping_add(1);
    }
}
