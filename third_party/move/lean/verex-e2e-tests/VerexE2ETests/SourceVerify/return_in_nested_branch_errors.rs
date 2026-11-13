// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A `return` closing a branch nested in a statement leaves the function: read as
// the branch's value, the loop would go on and no result would follow.

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
