// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Nothing here assumes something false: `ensures false` fails on each.

pub struct Guard(pub u32);

impl Drop for Guard {
    fn drop(&mut self) {}
}

pub fn make(value: u32) -> Box<u32> {
    Box::new(value)
}

pub fn read(value: Box<u32>) -> u32 {
    *value
}

pub fn read_nested(value: Box<Box<u32>>) -> u32 {
    **value
}

pub fn read_field(value: &(u32, Box<(u32, u32)>)) -> u32 {
    value.1.0
}

pub fn move_out(value: Box<Vec<u32>>) -> Vec<u32> {
    *value
}

pub fn move_guard(value: Box<Guard>) -> Guard {
    *value
}

pub fn move_part(value: Box<(Vec<u32>, u32)>) -> Vec<u32> {
    value.0
}

pub fn move_behind_flag(flag: bool, value: Box<Vec<u32>>) -> usize {
    if flag {
        let content = *value;
        content.len()
    } else {
        0
    }
}

pub fn write(value: &mut Box<u32>, other: u32) {
    **value = other;
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

pub fn borrow_mut(value: &mut Box<u32>) -> &mut u32 {
    &mut **value
}

pub fn view(value: &Box<u32>) -> &u32 {
    &**value
}

pub fn as_ref(value: &Box<u32>) -> &u32 {
    value.as_ref()
}

pub fn as_mut(value: &mut Box<u32>, other: u32) {
    *value.as_mut() = other;
}

pub fn dropped(_value: Box<u32>) -> u32 {
    1
}

pub fn dropped_holder(_value: Box<Vec<u32>>) -> u32 {
    1
}
