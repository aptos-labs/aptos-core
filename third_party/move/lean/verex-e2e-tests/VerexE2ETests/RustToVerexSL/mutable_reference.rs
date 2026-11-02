// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn replace(value: &mut u32, replacement: u32) -> u32 {
    *value = replacement;
    *value
}
