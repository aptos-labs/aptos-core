// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn last_or_zero(values: &[u32]) -> u32 {
    let [.., last] = values else {
        return 0;
    };
    *last
}
