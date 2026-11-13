// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Pair(pub u64, pub u64);

pub trait Limit {
    const MAX: u64;
}

pub fn promoted() -> &'static u64 {
    &5
}

pub fn generic_limit<T: Limit>() -> u64 {
    T::MAX
}

pub fn generic_clone<T: Clone>(value: &T) -> T {
    value.clone()
}

pub fn reads_promoted() -> u64 {
    *promoted()
}

pub fn reads_limit() -> u64 {
    plain(1)
}

pub fn plain(a: u64) -> u64 {
    a / 2
}
