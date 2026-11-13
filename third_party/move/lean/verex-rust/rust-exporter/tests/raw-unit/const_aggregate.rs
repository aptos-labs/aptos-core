// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Point {
    pub x: u32,
    pub flag: bool,
}

pub enum Level {
    Low,
    High(u16),
}

const ORIGIN: Point = Point { x: 3, flag: true };
const LEVEL: Level = Level::High(9);
const PAIR: (u8, u64) = (1, 2);
const TABLE: [u16; 3] = [10, 20, 30];
const SOME: Option<u32> = Some(7);
const NAMED: Option<&str> = Some("name");

pub fn origin() -> Point {
    ORIGIN
}

pub fn level() -> Level {
    LEVEL
}

pub fn pair() -> (u8, u64) {
    PAIR
}

pub fn table(index: usize) -> u16 {
    TABLE[index]
}

pub fn some() -> Option<u32> {
    SOME
}

pub fn named() -> Option<&'static str> {
    NAMED
}
