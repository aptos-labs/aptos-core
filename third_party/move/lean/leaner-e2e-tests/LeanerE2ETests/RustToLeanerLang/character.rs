// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn ordering(left: char, right: char) -> (bool, bool, bool, bool) {
    (left < right, left <= right, left > right, left >= right)
}

pub fn literal() -> char {
    '🦀'
}

pub fn classify(value: char) -> u8 {
    match value {
        'a' => 1,
        '🦀' => 2,
        _ => 0,
    }
}

pub fn to_u32(value: char) -> u32 {
    value as u32
}

pub fn from_ascii(value: u8) -> char {
    value as char
}
