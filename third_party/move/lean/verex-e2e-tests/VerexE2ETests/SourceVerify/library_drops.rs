// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Holder {
    pub values: Vec<u32>,
}

pub struct Generic<T> {
    pub values: Vec<T>,
}

pub struct Outer {
    pub inner: Holder,
    pub rows: Vec<Vec<u32>>,
}

pub enum Choice {
    Empty,
    Some(Vec<u32>),
}

pub fn holder(_value: Holder) -> u32 {
    1
}

pub fn generic(_value: Generic<u32>) -> u32 {
    1
}

pub fn outer(_value: Outer) -> u32 {
    1
}

pub fn choice(_value: Choice) -> u32 {
    1
}

pub fn nested(_value: Vec<Vec<u32>>) -> u32 {
    1
}

pub fn of_holders(_value: Vec<Holder>) -> u32 {
    1
}

pub fn optional(_value: Option<Vec<u32>>) -> u32 {
    1
}

pub fn generic_nested(_value: Vec<Generic<u32>>) -> u32 {
    1
}
