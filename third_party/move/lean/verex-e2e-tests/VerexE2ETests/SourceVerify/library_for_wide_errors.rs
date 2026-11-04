// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn count(n: u64) -> u64 {
    let mut total = 0;
    for _i in 0..n {
        total += 1;
    }
    total
}
