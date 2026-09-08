// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Wrapper<T> {
    pub value: T,
}

pub fn unwrap_u32(value: Wrapper<u32>) -> u32 {
    value.value
}

