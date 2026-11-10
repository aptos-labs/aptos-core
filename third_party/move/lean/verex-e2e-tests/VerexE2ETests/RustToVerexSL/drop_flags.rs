// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A value moved on some paths only is dropped under a flag that records
// whether it is still owned.

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
