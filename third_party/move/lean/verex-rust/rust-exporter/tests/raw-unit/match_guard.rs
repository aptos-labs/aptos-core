// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub enum Maybe {
    None,
    Some(u32),
}

pub fn positive_or(value: Maybe, fallback: u32) -> u32 {
    match value {
        Maybe::Some(candidate) if candidate > 0 => candidate,
        _ => fallback,
    }
}
