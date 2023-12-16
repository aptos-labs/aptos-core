// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    constants::BLOB_STORAGE_SIZE,
    storage_format::{CacheEntry, CacheEntryBuilder, CacheEntryKey, StorageFormat},
};
use anyhow::Context;
use aptos_protos::transaction::v1::Transaction;
use redis::{AsyncCommands, RedisError, RedisResult};

// Configurations for cache.
// Cache entries that are present.
const CACHE_SIZE_ESTIMATION: u64 = 250_000_u64;

// Hard limit for cache lower bound. Only used for active eviction.
// Cache worker actively evicts the cache entries if the cache entry version is
// lower than the latest version - CACHE_SIZE_EVICTION_LOWER_BOUND.
// The gap between CACHE_SIZE_ESTIMATION and this is to give buffer since
// reading latest version and actual data not atomic(two operations).
const CACHE_SIZE_EVICTION_LOWER_BOUND: u64 = 300_000_u64;

// Keys for cache.
const CACHE_KEY_LATEST_VERSION: &str = "latest_version";
const CACHE_KEY_CHAIN_ID: &str = "chain_id";
// 9999-12-31 23:59:59. UTC.
const BASE_EXPIRATION_EPOCH_TIME_IN_SECONDS: u64 = 253_402_300_799;

// Default values for cache.
const CACHE_DEFAULT_LATEST_VERSION_NUMBER: &str = "0";
const FILE_STORE_LATEST_VERSION: &str = "file_store_latest_version";

/// This Lua script is used to update the latest version in cache.
///   Returns 0 if the cache is updated to 0 or sequentially update.
///   Returns 1 if the cache is not updated and gap detected.
const CACHE_SCRIPT_UPDATE_LATEST_VERSION_WITH_CHECK: &str = r#"
    local latest_version = redis.call("GET", KEYS[1])
    local start_version = tonumber(ARGV[1])
    local end_version_inclusive = tonumber(ARGV[2])
    if latest_version then
        if tonumber(latest_version) + 1 < start_version then
            return 1
        else
            redis.call("SET", KEYS[1], math.max(tonumber(latest_version), end_version_inclusive))
            return 0
        end
    else
        redis.call("SET", KEYS[1], end_version_inclusive)
        return 0
    end
"#;

#[derive(Debug, Clone)]
pub enum CacheBatchGetStatus {
    /// OK with batch of encoded transactions.
    Ok(Vec<Transaction>),
    /// Requested version is already evicted from cache. Visit file store instead.
    EvictedFromCache,
    /// Not ready yet. Wait and retry.
    NotReady,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheUpdateStatus {
    /// 0 - Cache is updated from version x to x + 1. New key `x+1` with corresponding encoded data is added.
    Ok,
    /// 1 - Cache is not updated because current version is ahead of the latest version.
    AheadOfLatestVersion,
    /// 2 - Cache is not updated but verified. This is the case when the cache is updated by other workers from an old version.
    VerifiedWithoutUpdate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CacheCoverageStatus {
    /// Requested version is not processed by cache worker yet.
    DataNotReady,
    /// Requested version is cached.
    /// Transactions are available in cache: [requested_version, requested_version + value).
    CacheHit(u64),
    /// Requested version is evicted from cache.
    CacheEvicted,
}

/// Get the TTL in seconds for a given timestamp.
pub fn get_ttl_in_seconds(timestamp_in_seconds: u64) -> u64 {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    BASE_EXPIRATION_EPOCH_TIME_IN_SECONDS - (current_time - timestamp_in_seconds)
}

// Cache operator directly interacts with redis conn.
#[derive(Clone)]
pub struct CacheOperator<T: redis::aio::ConnectionLike + Send> {
    conn: T,
    storage_format: StorageFormat,
}

impl<T: redis::aio::ConnectionLike + Send + Clone> CacheOperator<T> {
    pub fn new(conn: T, storage_format: StorageFormat) -> Self {
        Self {
            conn,
            storage_format,
        }
    }

    // Set up the cache if needed.
    pub async fn cache_setup_if_needed(&mut self) -> anyhow::Result<bool> {
        let version_inserted: bool = redis::cmd("SET")
            .arg(CACHE_KEY_LATEST_VERSION)
            .arg(CACHE_DEFAULT_LATEST_VERSION_NUMBER)
            .arg("NX")
            .query_async(&mut self.conn)
            .await
            .context("Redis latest_version check failed.")?;
        if version_inserted {
            tracing::info!(
                initialized_latest_version = CACHE_DEFAULT_LATEST_VERSION_NUMBER,
                "Cache latest version is initialized."
            );
        }
        Ok(version_inserted)
    }

    pub async fn set_chain_id(&mut self, chain_id: u64) -> anyhow::Result<()> {
        self.conn
            .set(CACHE_KEY_CHAIN_ID, chain_id)
            .await
            .context("Redis chain id update failed.")?;
        Ok(())
    }

    pub async fn get_chain_id(&mut self) -> anyhow::Result<Option<u64>> {
        self.get_config_by_key(CACHE_KEY_CHAIN_ID).await
    }

    pub async fn get_latest_version(&mut self) -> anyhow::Result<Option<u64>> {
        self.get_config_by_key(CACHE_KEY_LATEST_VERSION).await
    }

    pub async fn get_file_store_latest_version(&mut self) -> anyhow::Result<Option<u64>> {
        self.get_config_by_key(FILE_STORE_LATEST_VERSION).await
    }

    /// This gets latest version, chain id, and file store latest version
    async fn get_config_by_key(&mut self, key: &str) -> anyhow::Result<Option<u64>> {
        let result = self.conn.get::<&str, Vec<u8>>(key).await?;
        if result.is_empty() {
            Ok(None)
        } else {
            let result_string = String::from_utf8(result).unwrap();
            Ok(Some(result_string.parse::<u64>().with_context(|| {
                format!("Redis key {} is not a number.", key)
            })?))
        }
    }

    pub async fn update_file_store_latest_version(
        &mut self,
        latest_version: u64,
    ) -> anyhow::Result<()> {
        self.conn
            .set(FILE_STORE_LATEST_VERSION, latest_version)
            .await?;
        Ok(())
    }

    // Internal function to get the latest version from cache.
    pub(crate) async fn check_cache_coverage_status(
        &mut self,
        requested_version: u64,
    ) -> anyhow::Result<CacheCoverageStatus> {
        let latest_version: u64 = match self
            .conn
            .get::<&str, String>(CACHE_KEY_LATEST_VERSION)
            .await
        {
            Ok(v) => v
                .parse::<u64>()
                .expect("Redis latest_version is not a number."),
            Err(err) => return Err(err.into()),
        };

        if requested_version >= latest_version {
            Ok(CacheCoverageStatus::DataNotReady)
        } else if requested_version + CACHE_SIZE_ESTIMATION < latest_version {
            Ok(CacheCoverageStatus::CacheEvicted)
        } else {
            // TODO: rewrite this logic to surface this max fetch size better
            Ok(CacheCoverageStatus::CacheHit(std::cmp::min(
                latest_version - requested_version,
                BLOB_STORAGE_SIZE as u64,
            )))
        }
    }

    pub async fn update_cache_transactions(
        &mut self,
        transactions: Vec<Transaction>,
    ) -> anyhow::Result<usize> {
        let cache_entries: Vec<(u64, String, Vec<u8>, u64)> = transactions
            .iter()
            .map(|transaction| {
                let version = transaction.version;
                let cache_key_builder =
                    CacheEntryKey::new(transaction.version, self.storage_format);
                let cache_key = cache_key_builder.to_string();
                let timestamp_in_seconds = match transaction.timestamp {
                    Some(ref timestamp) => timestamp.seconds as u64,
                    None => 0,
                };
                let cache_entry_builder =
                    CacheEntryBuilder::new(transaction.clone(), self.storage_format);
                let cache_entry: CacheEntry = cache_entry_builder
                    .try_into()
                    .expect("Serialization failed.");
                let encoded_cache_entry = cache_entry.into_inner();
                (
                    version,
                    cache_key,
                    encoded_cache_entry,
                    timestamp_in_seconds,
                )
            })
            .collect();

        let serialized_size = cache_entries
            .iter()
            .map(|(_, _, encoded, _)| encoded.len())
            .sum::<usize>();

        let mut redis_pipeline = redis::pipe();
        for (version, key, encoded_proto_data, timestamp_in_seconds) in cache_entries {
            redis_pipeline
                .cmd("SET")
                .arg(key)
                .arg(encoded_proto_data)
                .arg("EX")
                .arg(get_ttl_in_seconds(timestamp_in_seconds))
                .ignore();
            // Actively evict the expired cache. This is to avoid using Redis
            // eviction policy, which is probabilistic-based and may evict the
            // cache that is still needed.
            if version >= CACHE_SIZE_EVICTION_LOWER_BOUND {
                redis_pipeline
                    .cmd("DEL")
                    .arg(version - CACHE_SIZE_EVICTION_LOWER_BOUND)
                    .ignore();
            }
        }

        let redis_result: RedisResult<()> =
            redis_pipeline.query_async::<_, _>(&mut self.conn).await;

        match redis_result {
            Ok(_) => Ok(serialized_size),
            Err(err) => Err(err.into()),
        }
    }

    // Overwrite the latest version in cache.
    // Only call this function during cache worker startup.
    pub async fn update_cache_latest_version(&mut self, version: u64) -> anyhow::Result<()> {
        self.conn
            .set(CACHE_KEY_LATEST_VERSION, version)
            .await
            .context("Redis latest version overwrite failed.")?;
        Ok(())
    }

    // Update the latest version in cache.
    pub async fn update_cache_latest_version_with_check(
        &mut self,
        start_version: u64,
        end_version_inclusive: u64,
    ) -> anyhow::Result<()> {
        let script = redis::Script::new(CACHE_SCRIPT_UPDATE_LATEST_VERSION_WITH_CHECK);
        tracing::debug!(
            start_version = start_version,
            end_version_inclusive = end_version_inclusive,
            "Updating latest version in cache."
        );
        match script
            .key(CACHE_KEY_LATEST_VERSION)
            .arg(start_version)
            .arg(end_version_inclusive)
            .invoke_async(&mut self.conn)
            .await
            .context("Redis latest version update failed.")?
        {
            1 => {
                tracing::error!(
                    end_version_inclusive=end_version_inclusive,
                    start_version=start_version,
                    "Redis latest version update failed. The version is beyond the next expected version.");
                Err(anyhow::anyhow!("Version is not right."))
            },
            _ => Ok(()),
        }
    }

    // TODO: Remove this
    pub async fn batch_get_encoded_proto_data(
        &mut self,
        start_version: u64,
    ) -> anyhow::Result<CacheBatchGetStatus> {
        let cache_coverage_status = self.check_cache_coverage_status(start_version).await;
        match cache_coverage_status {
            Ok(CacheCoverageStatus::CacheHit(v)) => {
                let cache_keys = (start_version..start_version + v)
                    .map(|e| CacheEntryKey::new(e, self.storage_format).to_string())
                    .collect::<Vec<String>>();
                let encoded_transactions: Result<Vec<Vec<u8>>, RedisError> =
                    self.conn.mget(cache_keys).await;
                match encoded_transactions {
                    Ok(v) => {
                        let transactions: Vec<Transaction> = v
                            .into_iter()
                            .map(|e| {
                                let cache_entry: CacheEntry =
                                    CacheEntry::from_bytes(e, self.storage_format);
                                let transaction: Transaction =
                                    cache_entry.try_into().expect("Deserialization failed.");
                                transaction
                            })
                            .collect();
                        Ok(CacheBatchGetStatus::Ok(transactions))
                    },
                    Err(err) => Err(err.into()),
                }
            },
            Ok(CacheCoverageStatus::CacheEvicted) => Ok(CacheBatchGetStatus::EvictedFromCache),
            Ok(CacheCoverageStatus::DataNotReady) => Ok(CacheBatchGetStatus::NotReady),
            Err(err) => Err(err),
        }
    }

    /// Fail if not all transactions requested are returned
    pub async fn batch_get_transactions(
        &mut self,
        start_version: u64,
        transaction_count: u64,
    ) -> anyhow::Result<Vec<Transaction>> {
        let cache_keys = (start_version..start_version + transaction_count)
            .map(|e| CacheEntryKey::new(e, self.storage_format).to_string())
            .collect::<Vec<String>>();
        let encoded_transactions: Result<Vec<Vec<u8>>, RedisError> =
            self.conn.mget(cache_keys).await;
        match encoded_transactions {
            Ok(txns) => {
                let transactions: Vec<Transaction> = txns
                    .into_iter()
                    .map(|e| {
                        let cache_entry: CacheEntry =
                            CacheEntry::from_bytes(e, self.storage_format);
                        let transaction: Transaction =
                            cache_entry.try_into().expect("Deserialization failed.");
                        transaction
                    })
                    .collect();
                Ok(transactions)
            },
            Err(err) => Err(err.into()),
        }
    }
}
