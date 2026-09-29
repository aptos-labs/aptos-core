// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Wrapper<T> {
    pub value: T,
}

pub struct Outer<T> {
    pub inner: Wrapper<T>,
}

pub fn read_u32(value: Outer<u32>) -> u32 {
    value.inner.value
}
