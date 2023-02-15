// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod storage;

pub const CACHE_KEY_CHAIN_ID: &str = "chain_id";
pub const BLOB_STORAGE_SIZE: u64 = 1_000;

/// Get redis address from env variable.
#[inline]
pub fn get_redis_address() -> String {
    std::env::var("REDIS_ADDRESS").expect("REDIS_ADDRESS is not set.")
}

#[inline]
pub fn get_file_store_bucket_name() -> String {
    let bucket_prefix =
        std::env::var("FILE_STORE_BUCKET_NAME").expect("FILE_STORE_BUCKET_NAME is not set.");
    let chain_name = std::env::var("CHAIN_NAME").expect("CHAIN_NAME is not set.");
    format!("{}-{}", bucket_prefix, chain_name)
}

#[inline]
pub fn get_health_check_port() -> u16 {
    std::env::var("HEALTH_CHECK_PORT").map_or_else(|_| 8080, |v| v.parse::<u16>().unwrap())
}
