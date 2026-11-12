// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

static mut COUNT: u64 = 0;

pub fn count() -> u64 {
    unsafe { COUNT }
}
