// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Point {
    pub x: u32,
}

static COUNT: u64 = 7;
static ORIGIN: Point = Point { x: 3 };

pub fn count() -> u64 {
    COUNT
}

pub fn origin() -> u32 {
    ORIGIN.x
}

static SHARED: &u64 = &COUNT;

pub fn shared() -> u64 {
    *SHARED
}
