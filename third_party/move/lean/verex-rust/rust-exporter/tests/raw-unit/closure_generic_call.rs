// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn apply<F: Fn(u32) -> u32>(f: F, x: u32) -> u32 {
    f(x)
}

pub fn use_apply(c: u32) -> u32 {
    apply(|x| x + c, 3)
}
