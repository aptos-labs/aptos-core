// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn powered(left: u32, right: u32) -> u32 {
    left.rotate_left(right)
}

pub fn wrap(value: u32) -> Option<u32> {
    Some(value)
}
