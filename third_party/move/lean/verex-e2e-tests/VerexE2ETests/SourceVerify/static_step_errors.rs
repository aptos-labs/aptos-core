// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub trait Step {
    fn step(&mut self);
}

pub fn step_twice<T: Step>(value: &mut T) {
    value.step();
    value.step();
}

pub fn forward<T: Step>(value: &mut T) {
    step_twice(value);
}

pub struct Counter {
    pub value: u64,
}

impl Step for Counter {
    fn step(&mut self) {
        self.value = self.value + 1;
    }
}

pub struct Toggle {
    pub value: bool,
}

impl Step for Toggle {
    fn step(&mut self) {
        self.value = !self.value;
    }
}

pub fn counter(value: u64) -> u64 {
    let mut state = Counter { value };
    step_twice(&mut state);
    state.value
}

pub fn toggle(value: bool) -> bool {
    let mut state = Toggle { value };
    forward(&mut state);
    state.value
}
