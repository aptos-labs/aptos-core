// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

const LIMIT: usize = 1024;

pub fn below_limit(value: usize) -> bool {
    value < LIMIT
}
