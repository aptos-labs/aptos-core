// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Leaf {
    pub value: u32,
}

pub struct Pair {
    pub first: Leaf,
    pub second: Leaf,
}

pub fn take_first(pair: Pair) -> Leaf {
    pair.first
}

