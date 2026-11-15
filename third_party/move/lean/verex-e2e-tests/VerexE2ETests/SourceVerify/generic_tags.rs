// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A type argument naming a mutable reference its values do not hold: reading the callee at it
// agrees with substituting it, so the callee's generic contract transfers.

use std::marker::PhantomData;

pub struct Tag<T> {
    pub tag: u8,
    pub marker: PhantomData<T>,
}

pub fn push_only<'a>(tags: &mut Vec<Tag<&'a mut u32>>, tag: Tag<&'a mut u32>) {
    tags.push(tag);
}

pub fn recurse<T>(x: T, n: u8) -> T {
    if n == 0 {
        x
    } else {
        recurse(x, n - 1)
    }
}

pub fn recurse_tag(v: u8) -> u8 {
    let tag: Tag<&mut u32> = recurse(Tag { tag: v, marker: PhantomData }, 0);
    tag.tag
}
