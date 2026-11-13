// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn get(values: [u32; 4], index: usize) -> u32 { values[index] }
pub fn first_of_pair(left: u32, right: u32) -> u32 {
    let pair = [left, right];
    pair[0]
}
