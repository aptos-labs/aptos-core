// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Counter {
    pub value: u32,
}

pub fn reset(counter: &mut Counter) {
    counter.value = 0;
}

pub fn reset_twice(counter: &mut Counter) {
    reset(counter);
    reset(counter);
}
