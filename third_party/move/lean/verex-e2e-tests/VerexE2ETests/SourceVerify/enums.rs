// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub enum Shape {
    Empty,
    Point(u32),
    Pair(u32, u32),
}

pub fn make_point(x: u32) -> Shape { Shape::Point(x) }

pub fn size(shape: &Shape) -> u32 {
    match shape {
        Shape::Empty => 0,
        Shape::Point(_) => 1,
        Shape::Pair(_, _) => 2,
    }
}

pub fn first(shape: Shape, fallback: u32) -> u32 {
    match shape {
        Shape::Empty => fallback,
        Shape::Point(x) => x,
        Shape::Pair(x, _) => x,
    }
}

pub fn sum(shape: Shape) -> u64 {
    match shape {
        Shape::Empty => 0,
        Shape::Point(x) => x as u64,
        Shape::Pair(x, y) => x as u64 + y as u64,
    }
}

pub struct Borrowed<'a, T> {
    pub value: &'a T,
}

pub fn read(borrowed: Borrowed<'_, u32>) -> u32 { *borrowed.value }

pub fn first_of(pair: (u32, bool)) -> u32 { pair.0 }

pub fn second_of(pair: &(u32, bool)) -> bool { pair.1 }

pub fn use_first(value: u32) -> u32 { first_of((value, true)) }

pub fn use_second(flag: bool) -> bool { second_of(&(1, flag)) }
