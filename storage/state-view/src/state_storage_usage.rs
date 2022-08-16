// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct StateStorageUsage {
    pub items: usize,
    pub bytes: usize,
}

impl StateStorageUsage {
    pub fn new(items: usize, bytes: usize) -> Self {
        Self { items, bytes }
    }

    pub fn zero() -> Self {
        Self::new(0, 0)
    }
}
