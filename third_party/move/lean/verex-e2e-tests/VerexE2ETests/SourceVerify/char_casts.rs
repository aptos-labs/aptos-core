// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn from_ascii(value: u8) -> char {
    value as char
}
pub fn code_of_ascii(value: u8) -> u32 {
    (value as char) as u32
}
pub fn is_letter(value: u8) -> bool {
    (value as char) == 'a'
}
pub fn low_byte(value: char) -> u8 {
    value as u8
}
pub fn bounded_code(value: char) -> u32 {
    value as u32
}
pub fn outside_surrogates(value: char) -> u32 {
    value as u32
}
