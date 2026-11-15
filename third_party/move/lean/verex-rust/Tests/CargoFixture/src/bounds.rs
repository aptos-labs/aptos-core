// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod compare {
    pub fn below(value: u32, limit: u32) -> bool {
        value < limit
    }
}

pub fn within(value: u32) -> bool {
    compare::below(value, crate::limit())
}
