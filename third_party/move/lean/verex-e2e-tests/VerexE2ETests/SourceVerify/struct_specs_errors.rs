// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Range {
    pub low: u32,
    pub high: u32,
}

pub fn invert(low: u32, high: u32) -> Range {
    Range { low: high, high: low }
}
