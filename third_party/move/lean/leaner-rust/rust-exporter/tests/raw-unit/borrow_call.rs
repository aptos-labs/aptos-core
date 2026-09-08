// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[inline(never)]
pub fn read(value: &u32) -> u32 {
    *value
}

pub fn borrow_then_read(value: u32) -> u32 {
    read(&value)
}
