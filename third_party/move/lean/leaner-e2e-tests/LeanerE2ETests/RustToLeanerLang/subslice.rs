// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn tail_len(values: &[u32]) -> usize {
    let [_, tail @ ..] = values else {
        return 0;
    };
    tail.len()
}
