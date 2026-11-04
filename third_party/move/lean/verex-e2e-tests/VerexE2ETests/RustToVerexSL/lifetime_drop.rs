// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Anonymous<'a> {
    pub r: &'a mut u32,
}

impl Drop for Anonymous<'_> {
    fn drop(&mut self) {
        *self.r = 9;
    }
}

pub struct Named<'a> {
    pub r: &'a mut u32,
}

impl<'a> Drop for Named<'a> {
    fn drop(&mut self) {
        *self.r = 9;
    }
}
