// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn first_pair(left: &u32, right: &u32) -> u32 {
    let pair = (left, right);
    *pair.0
}
