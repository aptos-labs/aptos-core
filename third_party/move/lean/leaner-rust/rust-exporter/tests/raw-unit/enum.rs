// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[repr(isize)]
pub enum Choice {
    First(u32) = 4,
    Second(u32) = 9,
}

pub fn first(value: u32) -> Choice {
    Choice::First(value)
}

pub fn select(value: Choice) -> u32 {
    match value {
        Choice::First(value) => value,
        Choice::Second(value) => value,
    }
}
