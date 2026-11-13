// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn clear(value: &mut u32) {
    *value = 0;
}

pub fn clear_twice(value: &mut u32) {
    clear(value);
    clear(value);
}
