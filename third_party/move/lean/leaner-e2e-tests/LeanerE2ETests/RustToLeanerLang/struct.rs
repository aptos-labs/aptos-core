// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
