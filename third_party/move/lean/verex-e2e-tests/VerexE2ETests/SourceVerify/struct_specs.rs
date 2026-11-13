// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Range {
    pub low: u32,
    pub high: u32,
}

pub fn widen(range: &mut Range) {
    if range.high < 1000 {
        range.high += 1;
    }
}

pub fn make(low: u32) -> Range {
    Range { low, high: low }
}
