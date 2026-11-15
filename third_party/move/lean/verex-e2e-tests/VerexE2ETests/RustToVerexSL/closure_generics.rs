// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn generic<T>(x: T) -> T {
    let g = move || x;
    g()
}

pub struct W<T> {
    pub v: T,
}

impl<T> W<T> {
    pub fn take(self) -> T {
        let g = move || self.v;
        g()
    }
}
