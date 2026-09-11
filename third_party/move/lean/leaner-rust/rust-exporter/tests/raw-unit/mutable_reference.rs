// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub fn replace(value: &mut u32, replacement: u32) -> u32 {
    *value = replacement;
    *value
}
