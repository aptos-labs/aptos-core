// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Tagged<T, const N: usize, const ENABLED: bool> {
    pub value: T,
}

pub fn read_u32(value: Tagged<u32, 3, true>) -> u32 {
    value.value
}
