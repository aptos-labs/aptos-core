// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::async_concurrent_dropper::AsyncConcurrentDropper;
use once_cell::sync::Lazy;

mod async_concurrent_dropper;
mod metrics;

pub static DEFAULT_DROPPER: Lazy<AsyncConcurrentDropper> =
    Lazy::new(|| AsyncConcurrentDropper::new("default", 32));
