// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn keep(a: u64, _tag: impl Copy) -> u64 {
    a
}

pub fn named_beside<Impl1>(x: Impl1, _tag: impl Copy) -> Impl1 {
    x
}
