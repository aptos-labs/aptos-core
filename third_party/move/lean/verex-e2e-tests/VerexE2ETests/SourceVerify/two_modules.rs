// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod m {
    pub struct Cell {
        pub value: u32,
    }

    pub fn cell(value: u32) -> Cell {
        Cell { value }
    }

    pub fn one() -> u32 {
        1
    }

    pub fn helper(value: u32) -> bool {
        value < 10
    }

    pub fn double(value: u32) -> u32 {
        super::n::add(value, value)
    }
}

pub mod n {
    pub struct Cell {
        pub value: u64,
    }

    pub fn cell(value: u64) -> Cell {
        Cell { value }
    }

    pub fn add(left: u32, right: u32) -> u32 {
        left + right
    }

    pub fn helper(value: u32) -> bool {
        value < 20
    }

    pub fn next(value: u32) -> u32 {
        add(value, super::m::one())
    }

    pub fn from_m(value: u32) -> u32 {
        super::m::cell(value).value
    }
}

pub fn both(value: u32) -> bool {
    m::helper(value) || n::helper(value)
}
