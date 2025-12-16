// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use aptos_drop_helper::async_concurrent_dropper::AsyncConcurrentDropper;
use once_cell::sync::Lazy;

pub static SUBTREE_DROPPER: Lazy<AsyncConcurrentDropper> =
    Lazy::new(|| AsyncConcurrentDropper::new("smt_subtree", 32, 8));
