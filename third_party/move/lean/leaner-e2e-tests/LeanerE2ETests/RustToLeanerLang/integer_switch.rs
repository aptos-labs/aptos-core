// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub fn select(tag: u32, zero: u32, one: u32, fallback: u32) -> u32 {
    match tag {
        0 => zero,
        1 => one,
        _ => fallback,
    }
}
