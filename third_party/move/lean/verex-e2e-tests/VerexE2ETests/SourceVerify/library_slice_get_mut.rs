// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn store(values: &mut [u32], i: usize, value: u32) -> bool {
    match values.get_mut(i) {
        Some(slot) => {
            *slot = value;
            true
        }
        None => false,
    }
}
