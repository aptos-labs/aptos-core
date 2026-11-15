// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Before the module.
pub mod m {
    /// The module's first function.
    pub fn one() -> u32 {
        1
    }

    // Between the functions of the module.
    pub fn two() -> u32 {
        // In a body.
        2
    }
}

/// A counter.
pub struct Counter {
    value: u32,
}

impl Counter {
    // Before a method.
    pub fn get(self) -> u32 {
        self.value
    }
}

// Before the root's function.
pub fn root() -> u32 {
    m::one()
}
