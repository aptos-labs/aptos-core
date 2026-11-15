// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn inner_local(n: u32) -> u32 {
    let mut i = 0;
    while i < n {
        let step = 1;
        i += step;
    }
    i
}
