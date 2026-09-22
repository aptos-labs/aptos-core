#![allow(dead_code)]

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub enum Choice {
    First(u32),
    Second(u32),
}

pub fn scalar(left: u32, right: u32) -> u32 {
    left + right
}

pub fn select(value: Choice) -> u32 {
    match value {
        Choice::First(value) => value,
        Choice::Second(value) => value + 1,
    }
}

#[inline(never)]
pub fn increment(value: u32) -> u32 {
    value + 1
}

pub fn direct_call(value: u32) -> u32 {
    increment(value)
}

pub fn count_to(limit: u32) -> u32 {
    let mut current = 0;
    while current < limit {
        current += 1;
    }
    current
}

#[inline(never)]
pub fn read(value: &u32) -> u32 {
    *value
}

pub fn ordinary_borrow(value: u32) -> u32 {
    read(&value)
}

pub struct Dropped(pub u32);

impl Drop for Dropped {
    fn drop(&mut self) {}
}

pub fn explicit_drop(value: Dropped) {
    drop(value);
}

pub fn implicit_drop(value: u32) {
    let _owned = Dropped(value);
}

#[inline(never)]
pub fn may_panic(value: u32) {
    assert!(value != 0);
}

pub fn cleanup_on_unwind(value: u32) {
    let _owned = Dropped(value);
    may_panic(value);
}

pub fn raw_pointer(value: &u32) -> *const u32 {
    value as *const u32
}
