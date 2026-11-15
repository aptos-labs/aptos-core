// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Counter {
    value: u32,
}

impl Counter {
    pub fn add(self, amount: u32) -> u32 {
        self.value + amount
    }

    pub fn start(value: u32) -> Counter {
        Counter { value }
    }
}

pub struct Gauge {
    value: u32,
}

impl Gauge {
    pub fn add(self, amount: u32) -> u32 {
        self.value + amount + 1
    }
}

pub fn call_method(value: u32, amount: u32) -> u32 {
    Counter::start(value).add(amount)
}

pub fn call_gauge(value: u32) -> u32 {
    Gauge { value }.add(1)
}
