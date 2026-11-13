// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Counter {
    pub value: u32,
}

impl Counter {
    pub fn get(self) -> u32 {
        self.value
    }
}
