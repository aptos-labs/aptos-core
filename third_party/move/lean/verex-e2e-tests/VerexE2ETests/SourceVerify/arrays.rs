// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn get(values: [u32; 4], index: usize) -> u32 { values[index] }
pub fn get_third(values: [u32; 4]) -> u32 { values[2] }
pub fn first_of_pair(left: u32, right: u32) -> u32 {
    let pair = [left, right];
    pair[0]
}
pub fn repeated(value: u32) -> u32 {
    let row = [value; 3];
    row[1]
}
pub fn set_first(mut values: [u32; 2], value: u32) -> u32 {
    values[0] = value;
    values[0]
}
