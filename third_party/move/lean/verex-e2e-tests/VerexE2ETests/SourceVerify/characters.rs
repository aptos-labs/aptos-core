// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn before(left: char, right: char) -> bool { left < right }
pub fn literal() -> char { 'a' }
pub fn same(left: char, right: char) -> bool { left == right }
pub fn classify(value: char) -> u8 {
    match value {
        'a' => 1,
        _ => 0,
    }
}
pub fn code(value: char) -> u32 { value as u32 }
pub fn from_ascii(value: u8) -> char { value as char }
