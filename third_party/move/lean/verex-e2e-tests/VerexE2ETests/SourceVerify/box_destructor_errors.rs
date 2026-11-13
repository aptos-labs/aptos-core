// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// What a box's drop runs is its content's destructor: a drop reaching one has no
// denotation, whole or behind the flag of a move out. A free alone runs none (`boxes`).

pub struct Guard(pub u32);

impl Drop for Guard {
    fn drop(&mut self) {}
}

pub struct Holder(pub Box<Guard>);

pub fn whole(_value: Box<Guard>) -> u32 {
    1
}

pub fn read_then_drop(value: Box<Guard>) -> u32 {
    value.0
}

pub fn in_a_field(_value: Holder) -> u32 {
    1
}

pub fn behind_flag(flag: bool, value: Box<Guard>) -> Option<Guard> {
    if flag { Some(*value) } else { None }
}

pub struct Pair {
    pub first: u32,
    pub second: Guard,
}

pub fn part_behind_flag(flag: bool, value: Box<Pair>) -> u32 {
    if flag {
        let Pair { first, second: _second } = *value;
        first
    } else {
        0
    }
}
