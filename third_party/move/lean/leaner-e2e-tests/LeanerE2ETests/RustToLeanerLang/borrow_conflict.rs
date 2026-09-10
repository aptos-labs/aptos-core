// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn conflicting_mutable_borrows(value: &mut u32) {
    let first = &mut *value;
    let second = &mut *value;
    *first += 1;
    *second += 1;
}
