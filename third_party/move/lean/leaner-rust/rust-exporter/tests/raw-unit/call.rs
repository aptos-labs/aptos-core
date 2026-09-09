// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[inline(never)]
pub fn recurse(again: bool, value: u32) -> u32 {
    if again { recurse(false, value) } else { value }
}
