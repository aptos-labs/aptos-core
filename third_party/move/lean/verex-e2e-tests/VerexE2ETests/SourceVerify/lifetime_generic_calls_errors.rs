// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Generic<'a, T> {
    pub r: &'a T,
}

impl<'a, T> Generic<'a, T> {
    pub fn inner(&self) -> &'a T {
        self.r
    }

    pub fn again(&self) -> &'a T {
        self.inner()
    }

    pub fn unspecified(&self) -> &'a T {
        self.r
    }
}

pub fn use_inner(h: &Generic<'_, u32>) -> u32 {
    *h.inner()
}

pub fn use_again(h: &Generic<'_, u32>) -> u32 {
    *h.again()
}

pub fn use_unspecified(h: &Generic<'_, u32>) -> u32 {
    *h.unspecified()
}
