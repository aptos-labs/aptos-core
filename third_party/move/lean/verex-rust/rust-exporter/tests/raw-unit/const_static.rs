// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Point {
    pub x: u32,
    pub label: &'static str,
}

static COUNT: u64 = 7;
static ORIGIN: Point = Point { x: 3, label: "origin" };
static SHARED: &u64 = &COUNT;

pub fn count() -> u64 {
    COUNT
}

pub fn count_reference() -> &'static u64 {
    &COUNT
}

pub fn origin() -> u32 {
    ORIGIN.x
}

pub fn label() -> &'static str {
    ORIGIN.label
}

pub fn shared() -> u64 {
    *SHARED
}
