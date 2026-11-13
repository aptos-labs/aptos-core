// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn identity<T>(value: T) -> T { value }

pub fn through_generic(pair: [u8; 2]) -> u8 {
    let held = identity(pair);
    held[1]
}
