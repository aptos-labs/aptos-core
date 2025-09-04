// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use velor_drop_helper::async_concurrent_dropper::AsyncConcurrentDropper;
use once_cell::sync::Lazy;

pub(crate) static DROPPER: Lazy<AsyncConcurrentDropper> =
    Lazy::new(|| AsyncConcurrentDropper::new("layered_map", 32, 8));
