// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Tuple support must not erase sharing inside the returned aggregate.
pub fn apply<'a, F: FnMut() -> (&'a u64, bool)>(callback: &mut F) -> (&'a u64, bool) {
    callback()
}

pub fn borrowed(value: &u64) -> (&u64, bool) {
    let mut callback = || (value, true);
    apply(&mut callback)
}
