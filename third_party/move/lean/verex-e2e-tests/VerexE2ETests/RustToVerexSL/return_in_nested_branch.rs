// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The `return` closes a branch nested in a statement: its value is not the branch's.
pub fn in_while(a: bool, b: bool, c: bool) -> u64 {
    let mut x = 0;
    while c {
        if a {
            if b {
                break;
            }
            x = 1;
            return x;
        }
        x = 2;
    }
    x
}

// The same with the `return` in a block of its own, folded with its value.
pub fn after_a_branch(a: bool, b: bool, c: bool, d: bool) -> u64 {
    let mut x = 0;
    while c {
        if a {
            if b {
                break;
            }
            if d {
                x = 1;
            }
            return x;
        }
        x = 2;
    }
    x
}

pub fn in_loop(a: bool, b: bool) -> u64 {
    let mut x = 0;
    loop {
        if a {
            if b {
                continue;
            }
            x = x + 1;
            return x;
        }
        x = 2;
    }
}
