// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub enum Shape {
    Empty,
    Point(u32),
}

pub fn size(shape: Shape) -> u32 {
    match shape {
        Shape::Empty => 0,
        Shape::Point(_) => 1,
    }
}

pub struct Borrowed<'a, T> {
    pub value: &'a T,
}

pub fn read(borrowed: Borrowed<'_, u32>) -> u32 { *borrowed.value }

pub fn first_of(pair: (u32, bool)) -> u32 { pair.0 }

pub fn use_first(value: u32) -> u32 { first_of((value, true)) }
