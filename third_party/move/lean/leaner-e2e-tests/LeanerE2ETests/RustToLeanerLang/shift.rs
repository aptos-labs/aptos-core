// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn shifts(value: u32, distance: u8) -> (u32, u32) {
    (value << distance, value >> distance)
}

