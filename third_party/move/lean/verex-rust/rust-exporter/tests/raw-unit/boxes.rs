// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A box is its content: read, moved out, written through, and dropped.

pub fn read(value: Box<u32>) -> u32 {
    *value
}

pub fn read_nested(value: Box<Box<u32>>) -> u32 {
    **value
}

pub fn read_field(value: &(u32, Box<(u32, u32)>)) -> u32 {
    value.1.0
}

pub fn move_out(value: Box<(u32, u32)>) -> (u32, u32) {
    *value
}

pub fn move_part(value: Box<(u32, u32)>, flag: bool) -> u32 {
    if flag { value.0 } else { value.1 }
}

pub fn write(value: &mut Box<u32>, other: u32) -> u32 {
    **value = other;
    **value
}

pub fn write_field(value: &mut (u32, Box<(u32, u32)>), other: u32) {
    value.1.1 = other;
}

pub fn write_through(value: &mut Box<u32>, other: u32) {
    let content = &mut **value;
    *content = other;
}

pub fn write_owned(mut value: Box<u32>, other: u32) -> u32 {
    *value = other;
    *value
}

pub fn dropped(_value: Box<u32>) -> u32 {
    1
}

pub fn make(value: u32) -> Box<u32> {
    Box::new(value)
}
