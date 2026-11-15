// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[derive(PartialEq, Clone, Copy)]
pub struct Point {
    pub a: u64,
    pub b: u64,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Color {
    Red,
    Green,
}

pub fn same_point(p: Point, q: Point) -> bool {
    p == q
}

pub fn same_color(x: Color, y: Color) -> bool {
    x == y
}

pub fn same_string(s: &str, t: &str) -> bool {
    s.len() == t.len()
}
