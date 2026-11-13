// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[derive(Clone, Copy)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

pub fn duplicate(point: Point) -> (Point, Point) {
    (point, point)
}

// `Option<T>` is `Copy` where `T` is, so `Option<u32>` is copied.
pub fn twice(value: Option<u32>) -> (Option<u32>, Option<u32>) {
    (value, value)
}
