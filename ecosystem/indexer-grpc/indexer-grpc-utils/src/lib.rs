// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use redis::Commands;

pub mod storage;

pub const CACHE_KEY_CHAIN_ID: &str = "chain_id";
const CACHE_LATEST_VERSION: &str = "latest_version";

const CACHE_SIZE_ESTIMATION: u64 = 10_000_000;

pub const BLOB_STORAGE_SIZE: u64 = 1_000;

/// Enum to indicate the cache status of the requested version.
pub enum CacheCoverageStatus {
    /// Requested version is not processed by cache worker yet.
    DataNotReady,
    /// Requested version is cached.
    CacheHit,
    /// Requested version is evicted from cache.
    CacheEvicted,
}

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

pub async fn update_cache_latest_version(
    conn: &mut impl redis::ConnectionLike,
    latest_version: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    conn.set(CACHE_LATEST_VERSION, latest_version)
        .map_err(|e| e.into())
}

/// Check the cache status of the requested version.
/// The CacheEvicted is only an estimation from current latest version.
pub async fn get_cache_coverage_status(
    conn: &mut impl redis::ConnectionLike,
    requested_version: u64,
) -> Result<CacheCoverageStatus, Box<dyn std::error::Error>> {
    let latest_version: u64 = match conn.get(CACHE_LATEST_VERSION) {
        Ok(v) => v,
        Err(err) => return Err(err.into()),
    };

    let request_batch_upper_bound = requested_version
        .checked_add(BLOB_STORAGE_SIZE)
        .expect("Version boundary calculation overflows.");
    // Estimated cache lower bound is the latest version minus the cache size estimation; default to
    // 0.
    let estimated_cache_lower_bound = latest_version.saturating_sub(CACHE_SIZE_ESTIMATION);
    if request_batch_upper_bound > latest_version {
        // The cache should contain [requested_version, requested_version + BLOB_STORAGE_SIZE),
        // if upper bound is not covered, then the data is not ready.
        Ok(CacheCoverageStatus::DataNotReady)
    } else if estimated_cache_lower_bound < requested_version {
        // If request version lower bound is smaller than latest version, it means the data is evicted.
        Ok(CacheCoverageStatus::CacheEvicted)
    } else {
        Ok(CacheCoverageStatus::CacheHit)
    }
}

pub async fn get_cache_transactions(
    conn: &mut impl redis::ConnectionLike,
    requested_version: u64,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let versions = (requested_version..requested_version + BLOB_STORAGE_SIZE)
        .into_iter()
        .map(|e| e.to_string())
        .collect::<Vec<String>>();
    conn.mget(versions).map_err(|e| e.into())
}
