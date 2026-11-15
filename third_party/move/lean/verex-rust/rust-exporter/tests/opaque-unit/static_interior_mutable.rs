// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::AtomicU64;

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn counter() -> &'static AtomicU64 {
    &COUNTER
}
