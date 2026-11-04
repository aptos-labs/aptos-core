// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Point {
    pub x: u32,
}

const NAME: &str = "name";
const VALUE: &u64 = &5;
const POINT: &Point = &Point { x: 3 };
const VALUES: &[u16] = &[1, 2];
const NESTED: &&str = &"inner";

pub fn literal() -> &'static str {
    "text"
}

pub fn name() -> &'static str {
    NAME
}

pub fn value() -> u64 {
    *VALUE
}

pub fn point() -> u32 {
    POINT.x
}

pub fn values() -> &'static [u16] {
    VALUES
}

pub fn nested() -> &'static str {
    NESTED
}
