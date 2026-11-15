// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn sum(n: u32) -> u32 {
    let mut total = 0;
    for i in 0..n {
        total += i;
    }
    total
}
