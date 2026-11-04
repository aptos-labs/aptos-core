// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A drop has no effect on the values a body reads: of a type parameter, under
// its drop flag, of a row of them, and of a type that holds only them.

pub fn pick<T>(first: bool, left: T, right: T) -> T {
    if first { left } else { right }
}

pub fn keep_first<T>(values: (T, T), swap: bool) -> T {
    let (left, right) = values;
    if swap { right } else { left }
}

pub fn replace_once<T>(count: u32, initial: T, next: T) -> T {
    let mut result = initial;
    let mut index = 0;
    while index < count {
        if index == 1 {
            result = next;
            break;
        }
        index += 1;
    }
    result
}

pub fn forget<T>(_value: T) {}

pub fn forget_row<T, U>(_values: (T, U)) {}

pub struct Holder<T>(pub T);

pub fn forget_holder<T>(_holder: Holder<T>) {}

pub fn second_of_holders<T>(first: Holder<T>, second: T) -> T {
    forget_holder(first);
    second
}

pub fn pick_number(first: bool, left: u32, right: u32) -> u32 {
    pick(first, left, right)
}

pub fn pick_reference<'a>(first: bool, left: &'a u32, right: &'a u32) -> u32 {
    *pick(first, left, right)
}
