// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::async_concurrent_dropper::AsyncConcurrentDropper;
use once_cell::sync::Lazy;

pub mod async_concurrent_dropper;
pub mod async_drop_queue;
mod metrics;

pub static DEFAULT_DROPPER: Lazy<AsyncConcurrentDropper> =
    Lazy::new(|| AsyncConcurrentDropper::new("default", 32, 8));
