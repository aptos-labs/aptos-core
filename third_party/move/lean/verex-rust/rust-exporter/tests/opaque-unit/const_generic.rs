// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub trait Limit {
    const MAX: u64;
}

pub fn limit<T: Limit>() -> u64 {
    T::MAX
}
