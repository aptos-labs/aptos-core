// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn two_loops(n: u32) -> u32 {
    let mut i = 0;
    while i < n {
        i += 1;
    }
    let mut j = 0;
    while j < n {
        j += 1;
    }
    i + j
}
