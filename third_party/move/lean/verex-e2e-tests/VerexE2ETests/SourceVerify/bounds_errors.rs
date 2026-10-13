// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn max(left: u32, right: u32) -> u32 {
    if left >= right { left } else { right }
}

pub fn min(left: u32, right: u32) -> u32 {
    if left <= right { left } else { right }
}

pub fn clamp(value: u32, low: u32, high: u32) -> u32 {
    max(low, min(value, high))
}

pub fn raise(slot: &mut u32, floor: u32) {
    if *slot < floor {
        *slot = floor;
    }
}
