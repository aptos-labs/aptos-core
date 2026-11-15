// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Shared<'a> {
    pub r: &'a u32,
}

pub struct Generic<'a, T> {
    pub r: &'a T,
}

impl<'a> Shared<'a> {
    pub fn get(&self) -> u32 {
        *self.r
    }

    pub fn inner(&self) -> &'a u32 {
        self.r
    }
}

impl<'a, T> Generic<'a, T> {
    pub fn inner(&self) -> &'a T {
        self.r
    }
}

pub fn shrink<'s, 'l: 's>(h: Shared<'l>) -> Shared<'s> {
    h
}

pub fn method(x: u32) -> u32 {
    let h = Shared { r: &x };
    h.get()
}

pub fn shrunk(x: u32) -> u32 {
    let h = shrink(Shared { r: &x });
    *h.r
}

pub fn generic(x: u32) -> u32 {
    let h = Generic { r: &x };
    *h.inner()
}
