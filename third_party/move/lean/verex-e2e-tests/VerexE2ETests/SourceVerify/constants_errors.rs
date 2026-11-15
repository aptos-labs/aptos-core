// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

const LIMIT: usize = 1024;

static COUNT: u64 = 7;

pub fn below_limit(value: usize) -> bool {
    value < LIMIT
}

pub fn maximum() -> u64 {
    u64::MAX
}

pub fn count() -> u64 {
    COUNT
}
