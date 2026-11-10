// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn set_first(values: &mut [u32], value: u32) {
    if values.len() > 0 {
        values[0] = value;
    }
}

pub fn array_update() -> u32 {
    let mut values = [1, 2];
    set_first(&mut values, 9);
    values[0]
}

pub fn array_length_after(values: &mut [u32; 2]) -> usize {
    set_first(values, 9);
    values.len()
}

pub fn reborrow(values: &mut [u32]) -> usize {
    let inner = &mut *values;
    set_first(inner, 9);
    values.len()
}

#[allow(unused_assignments)]
pub fn redirect<'a>(left: &'a mut [u32], right: &'a mut [u32]) -> &'a mut [u32] {
    let mut selected = left;
    selected = right;
    selected
}

pub fn sequential_reborrow(values: &mut [u32]) -> usize {
    let first = &mut *values;
    set_first(first, 7);
    let second = &mut *values;
    set_first(second, 9);
    values.len()
}

pub fn loop_reborrow(values: &mut [u32]) -> usize {
    let mut rounds = 0u32;
    while rounds < 2 {
        let inner = &mut *values;
        set_first(inner, 11);
        rounds += 1;
    }
    values.len()
}

pub fn redirect_nested<'a>(selected: &mut &'a mut [u32], replacement: &'a mut [u32]) {
    *selected = replacement;
}
