// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A closure's loop is not one of the enclosing function.

pub fn only_closure_loop(n: u32) -> u32 {
    let sum = |m: u32| {
        let mut s = 0;
        let mut i = 0;
        while i < m {
            s = s + 2;
            i = i + 1;
        }
        s
    };
    sum(n)
}
