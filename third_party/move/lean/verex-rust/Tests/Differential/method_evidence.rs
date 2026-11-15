// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A dictionary that can panic must propagate its failure through both generic
// calls and invocation. This is execution coverage, not a claim that it meets
// the verifier's stronger, non-panicking MethodContract.
trait Step {
    fn step(&mut self);
}

struct Counter(u8);

impl Step for Counter {
    fn step(&mut self) {
        self.0 += 1;
    }
}

fn twice<T: Step>(value: &mut T) {
    value.step();
    value.step();
}

fn forward<T: Step>(value: &mut T) {
    twice(value);
}

pub fn checked_twice(value: u8) -> u8 {
    let mut counter = Counter(value);
    forward(&mut counter);
    counter.0
}
