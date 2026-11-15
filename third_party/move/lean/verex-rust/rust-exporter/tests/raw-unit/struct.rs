// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Pair {
    pub first: u32,
    pub second: u32,
}

pub fn make(first: u32, second: u32) -> Pair {
    Pair { first, second }
}

pub fn second(pair: Pair) -> u32 {
    pair.second
}
