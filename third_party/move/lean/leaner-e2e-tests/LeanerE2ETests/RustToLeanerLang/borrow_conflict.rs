// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn conflicting_mutable_borrows(value: &mut u32) {
    let first = &mut *value;
    let second = &mut *value;
    *first += 1;
    *second += 1;
}
