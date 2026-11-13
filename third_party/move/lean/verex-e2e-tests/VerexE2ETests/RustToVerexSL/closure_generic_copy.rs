// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Reading a `T: Copy` upvar: the closure body shows the bound.
pub fn generic<T: Copy>(x: T) -> T {
    let g = || x;
    g()
}
