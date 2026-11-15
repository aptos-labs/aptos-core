// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn bump(value: Option<u32>) -> Option<u32> {
    value.map(|x| x.wrapping_add(1))
}

pub fn constant(value: Option<u32>) -> Option<u32> {
    value.map(|_x| 7)
}
