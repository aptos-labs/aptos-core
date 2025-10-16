// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub enum StateStorageUsage {
    Tracked { items: usize, bytes: usize },
    Untracked,
}

impl StateStorageUsage {
    pub fn new(items: usize, bytes: usize) -> Self {
        Self::Tracked { items, bytes }
    }

    pub fn zero() -> Self {
        Self::new(0, 0)
    }

    pub fn new_untracked() -> Self {
        Self::Untracked
    }

    pub fn is_untracked(&self) -> bool {
        matches!(self, Self::Untracked)
    }

    pub fn items(&self) -> usize {
        match self {
            Self::Tracked { items, .. } => *items,
            Self::Untracked => 0,
        }
    }

    pub fn bytes(&self) -> usize {
        match self {
            Self::Tracked { bytes, .. } => *bytes,
            Self::Untracked => 0,
        }
    }

    pub fn add_item(&mut self, bytes_delta: usize) {
        match self {
            Self::Tracked { items, bytes } => {
                *items += 1;
                *bytes += bytes_delta;
            },
            Self::Untracked => (),
        }
    }

    pub fn remove_item(&mut self, bytes_delta: usize) {
        match self {
            Self::Tracked { items, bytes } => {
                *items -= 1;
                *bytes -= bytes_delta;
            },
            Self::Untracked => (),
        }
    }
}
