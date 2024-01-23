// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use once_cell::sync::Lazy;

pub(crate) static DRIVER_THREAD_POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .thread_name(|index| format!("driver_pool_{}", index))
        .build()
        .unwrap()
});

mod bootstrapper;
mod continuous_syncer;
mod driver;
mod driver_client;
pub mod driver_factory;
mod error;
mod logging;
pub mod metadata_storage;
pub mod metrics;
mod notification_handlers;
mod storage_synchronizer;
mod utils;

#[cfg(test)]
mod tests;
