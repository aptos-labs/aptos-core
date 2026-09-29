// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn select(tag: u32, zero: u32, one: u32, fallback: u32) -> u32 {
    match tag {
        0 => zero,
        1 => one,
        _ => fallback,
    }
}
