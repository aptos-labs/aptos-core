// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Counter {
    value: u32,
}

impl Counter {
    pub fn add(self, amount: u32) -> u32 {
        self.value + amount
    }
}

pub fn call_method(value: u32, amount: u32) -> u32 {
    Counter { value }.add(amount)
}
