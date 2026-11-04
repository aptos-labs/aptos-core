// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A value computed before a drop stays before it, also through a cast.

pub fn tail_cast(tags: Vec<u32>) -> u64 {
    tags.len() as u64
}

pub fn tail_truncate(tags: Vec<u32>) -> u8 {
    tags.len() as u8
}
