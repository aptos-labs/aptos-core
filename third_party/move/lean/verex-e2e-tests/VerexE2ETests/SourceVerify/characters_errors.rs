// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn literal() -> char { 'a' }
pub fn classify(value: char) -> u8 {
    match value {
        'a' => 1,
        _ => 0,
    }
}
