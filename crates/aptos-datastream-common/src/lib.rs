// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// The keys used in cache.
pub const CACHE_KEY_RUNNING_MODE: &str = "running_mode";
pub const CACHE_KEY_LATEST_VERSION: &str = "latest_version";
pub const CACHE_KEY_COLD_STORE_VERSION: &str = "cold_store_version";
pub const CACHE_KEY_CHAIN_ID: &str = "chain_id";
pub const CACHE_KEY_CHAIN_NAME: &str = "chain_name";

#[derive(Serialize, Deserialize)]
pub enum RunningMode {
    /// It's ok to have an empty RunningMode, or NULL in redis. Cold storage worker will flip to Recovery
    /// or Bootstrap mode based on existence of blob storage. At the same time, cache worker will pause.
    Default,

    /// The cold store will process all the transactions newly added to the cache.
    Normal,

    /// This is a special mode that is used to recover the cache and instruct the cache worker to restart.
    /// In this mode, the cold store will not process any transactions in cache. Instead, it will only push the
    /// recent transactions in cold storaeg into cache.
    ///
    /// Flip to normal mode when all recent transaction ingested, which allows the cache worker to
    /// resume from the next version.
    Recovery,

    /// In this mode, the cold store will process all the
    /// transactions up to head from genesis and then switch to normal mode.
    Bootstrap,

    /// Both the cache and cold store workers will be paused. This is used for maintenance.
    /// Flip the mode manually to Recovery once the maintenance is done.
    Maintenance,
}

#[derive(Serialize, Deserialize)]
pub struct ColdStoreTransactions {
    /// The version of the first transaction in the blob.
    /// This includes transactions [start_version, start_version + 100).
    /// It's guaranteed that `start_version % 100 == 0`
    start_version: u64,
    /// The list of transactions in base64 encoding.
    transactions: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ProcessedVersion {
    /// The version of the last processed transaction.
    version: u64,
    /// The time of the last update.
    update_time: u64,
}
