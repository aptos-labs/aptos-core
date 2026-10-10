// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Tagged<T, const N: usize, const ENABLED: bool> {
    pub value: T,
}

pub fn read_u32(value: Tagged<u32, 3, true>) -> u32 {
    value.value
}
