// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn fail(value: u64) -> u64 {
    if value == 0 {
        panic!("value {}", value);
    }
    value
}
