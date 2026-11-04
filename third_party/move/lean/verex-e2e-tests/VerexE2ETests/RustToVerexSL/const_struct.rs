// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Point {
    pub x: u32,
    pub flag: bool,
}

const ORIGIN: Point = Point { x: 3, flag: true };

pub fn origin() -> Point {
    ORIGIN
}
