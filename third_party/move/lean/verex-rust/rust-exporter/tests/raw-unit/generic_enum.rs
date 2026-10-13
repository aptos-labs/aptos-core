// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub enum Maybe<T> {
    None,
    Some(T),
}

pub fn unwrap_or(value: Maybe<u32>, fallback: u32) -> u32 {
    match value {
        Maybe::None => fallback,
        Maybe::Some(value) => value,
    }
}
