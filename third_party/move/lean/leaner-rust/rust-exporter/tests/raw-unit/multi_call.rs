// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[inline(never)]
pub fn transform(value: u32) -> u32 {
    value ^ 1
}

pub fn apply(value: u32) -> u32 {
    transform(value)
}
