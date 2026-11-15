// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn set_all(values: &mut [u32], value: u32) {
    let mut it = values.iter_mut();
    while let Some(x) = it.next() {
        *x = value;
    }
}
