// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub trait Step {
    fn step(&mut self);
}

pub fn step_twice<T: Step>(value: &mut T) {
    value.step();
    value.step();
}
