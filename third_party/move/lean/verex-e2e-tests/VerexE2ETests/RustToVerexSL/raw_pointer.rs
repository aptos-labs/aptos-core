// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn raw_pointer(value: &u32) -> *const u32 {
    value as *const u32
}
