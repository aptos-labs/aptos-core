// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_drop_helper::async_concurrent_dropper::AsyncConcurrentDropper;
use once_cell::sync::Lazy;

pub(crate) static SUBTREE_DROPPER: Lazy<AsyncConcurrentDropper> =
    Lazy::new(|| AsyncConcurrentDropper::new("smt_subtree", 32, 8));
