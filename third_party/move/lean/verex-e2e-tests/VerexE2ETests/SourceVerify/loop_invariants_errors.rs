// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn wrong_step(n: u32) -> u32 {
    let mut i = 0;
    while i < n {
        i += 1;
    }
    i
}

pub fn wrong_entry(n: u32) -> u32 {
    let mut i = 0;
    while i < n {
        i += 1;
    }
    i
}

pub fn wrong_inner(rows: u32, columns: u32) -> u32 {
    let mut row = 0;
    while row < rows {
        let mut column = 0;
        while column < columns {
            column += 1;
        }
        row += 1;
    }
    row
}

pub fn overflow(n: u32) -> u32 {
    let mut i = 0;
    let mut s = 0;
    while i < n {
        i += 1;
        s += 1000;
    }
    s
}

pub fn positive(x: u32) -> u32 {
    x
}

pub fn call_in_exiting_loop(n: u32) -> u32 {
    let mut i = 0;
    while i < n {
        positive(i);
        i += 1;
    }
    i
}

pub fn call_in_while(n: u32) -> u32 {
    let mut i = 0;
    while i < n {
        if i == 5 {
            break;
        }
        positive(i);
        i += 1;
    }
    i
}
