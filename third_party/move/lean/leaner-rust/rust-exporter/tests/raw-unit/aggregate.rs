// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn array(left: u32, right: u32) -> [u32; 2] {
    [left, right]
}

pub fn first(pair: (u32, bool)) -> u32 {
    pair.0
}

pub fn tuple(value: u32, flag: bool) -> (u32, bool) {
    (value, flag)
}
