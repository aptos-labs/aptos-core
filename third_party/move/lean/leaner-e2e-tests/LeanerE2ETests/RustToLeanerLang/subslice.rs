// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn tail_len(values: &[u32]) -> usize {
    let [_, tail @ ..] = values else {
        return 0;
    };
    tail.len()
}

