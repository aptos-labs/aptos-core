// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn pick<T>(first: bool, left: T, right: T) -> T {
    if first { left } else { right }
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
