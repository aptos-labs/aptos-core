// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Borrowed<'a, T> {
    pub value: &'a T,
}

pub fn read_u32(value: Borrowed<'_, u32>) -> u32 {
    *value.value
}
