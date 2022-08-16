// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum UsageInner {
    Tracked { items: usize, bytes: usize },
    Untracked,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct StateStorageUsage {
    inner: UsageInner,
}

impl StateStorageUsage {
    pub fn new(items: usize, bytes: usize) -> Self {
        Self {
            inner: UsageInner::Tracked { items, bytes },
        }
    }

    pub fn zero() -> Self {
        Self::new(0, 0)
    }

    pub fn new_untracked() -> Self {
        Self {
            inner: UsageInner::Untracked,
        }
    }

    pub fn is_untracked(&self) -> bool {
        matches!(self.inner, UsageInner::Untracked)
    }

    pub fn items(&self) -> usize {
        match self.inner {
            UsageInner::Tracked { items, .. } => items,
            UsageInner::Untracked => 0,
        }
    }

    pub fn bytes(&self) -> usize {
        match self.inner {
            UsageInner::Tracked { bytes, .. } => bytes,
            UsageInner::Untracked => 0,
        }
    }

    pub fn add_item(&mut self, bytes_delta: usize) {
        match self.inner {
            UsageInner::Tracked {
                ref mut items,
                ref mut bytes,
            } => {
                *items += 1;
                *bytes += bytes_delta;
            }
            UsageInner::Untracked => (),
        }
    }

    pub fn remove_item(&mut self, bytes_delta: usize) {
        match self.inner {
            UsageInner::Tracked {
                ref mut items,
                ref mut bytes,
            } => {
                *items -= 1;
                *bytes -= bytes_delta;
            }
            UsageInner::Untracked => (),
        }
    }
}
