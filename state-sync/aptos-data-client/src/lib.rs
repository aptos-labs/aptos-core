// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use once_cell::sync::Lazy;

#[allow(dead_code)]
pub(crate) static DATA_CLIENT_THREAD_POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .thread_name(|index| format!("data_client_pool_{}", index))
        .build()
        .unwrap()
});
pub mod client;
pub mod error;
pub mod global_summary;
pub mod interface;
mod latency_monitor;
mod logging;
mod metrics;
pub mod peer_states;
pub mod poller;
pub mod priority;
mod utils;

#[cfg(test)]
mod tests;
