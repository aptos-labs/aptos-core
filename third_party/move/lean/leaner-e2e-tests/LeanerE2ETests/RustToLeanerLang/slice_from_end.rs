// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn last_or_zero(values: &[u32]) -> u32 {
    let [.., last] = values else {
        return 0;
    };
    *last
}
