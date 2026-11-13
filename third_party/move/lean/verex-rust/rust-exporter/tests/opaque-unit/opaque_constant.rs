// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

fn increment(value: u64) -> u64 {
    value + 1
}

const HOOK: fn(u64) -> u64 = increment;

pub fn run(value: u64) -> u64 {
    HOOK(value)
}
