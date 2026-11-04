// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

const LIMIT: usize = 1024;
const NEGATIVE: i8 = -3;
const FLAG: bool = true;
const LETTER: char = 'x';

pub fn below_limit(value: usize) -> bool {
    value < LIMIT
}

pub fn maximum() -> u64 {
    u64::MAX
}

pub fn negative() -> i8 {
    NEGATIVE
}

pub fn flag() -> bool {
    FLAG
}

pub fn letter() -> char {
    LETTER
}

pub struct Scale;

impl Scale {
    const FACTOR: u64 = 3;

    pub fn factor() -> u64 {
        Self::FACTOR
    }
}
