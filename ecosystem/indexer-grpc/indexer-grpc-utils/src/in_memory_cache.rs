// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::compression_util::{CacheEntry, StorageFormat};
use anyhow::Context;
use velor_protos::transaction::v1::Transaction;
use dashmap::DashMap;
use itertools::Itertools;
use prost::Message;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

// Internal lookup retry interval for in-memory cache.
const IN_MEMORY_CACHE_LOOKUP_RETRY_INTERVAL_MS: u64 = 10;
const IN_MEMORY_CACHE_GC_INTERVAL_MS: u64 = 100;
// Max cache entry TTL: 30 seconds.
// const MAX_IN_MEMORY_CACHE_ENTRY_TTL: u64 = 30;
// Warm-up cache entries. Pre-fetch the cache entries to warm up the cache.
pub const WARM_UP_CACHE_ENTRIES: u64 = 100;
pub const MAX_REDIS_FETCH_BATCH_SIZE: usize = 500;
pub const MAX_FETCH_BATCH_SIZE: usize = 5000;

/// Configuration for when we want to explicitly declare how large the cache should be.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct InMemoryCacheSizeConfig {
    /// The maximum size of the cache in bytes.
    cache_target_size_bytes: u64,
    /// The maximum size of the cache in bytes before eviction is triggered, at which
    /// point we reduce the size of the cache back to `cache_target_size_bytes`.
    cache_eviction_trigger_size_bytes: u64,
}

impl Default for InMemoryCacheSizeConfig {
    fn default() -> Self {
        Self {
            // 3 GB.
            cache_target_size_bytes: 3_000_000_000,
            // 3.5 GB.
            cache_eviction_trigger_size_bytes: 3_500_000_000,
        }
    }
}

impl InMemoryCacheSizeConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.cache_target_size_bytes == 0 {
            return Err(anyhow::anyhow!("Cache target size must be greater than 0"));
        }
        if self.cache_eviction_trigger_size_bytes == 0 {
            return Err(anyhow::anyhow!(
                "Cache eviction trigger size must be greater than 0"
            ));
        }
        if self.cache_eviction_trigger_size_bytes < self.cache_target_size_bytes {
            return Err(anyhow::anyhow!(
                "Cache eviction trigger size must be greater than cache target size"
            ));
        }
        Ok(())
    }
}

/// Configuration for the in memory cache.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct InMemoryCacheConfig {
    size_config: InMemoryCacheSizeConfig,
}

impl InMemoryCacheConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        self.size_config.validate()
    }
}

#[derive(Debug, Clone, Copy)]
struct CacheMetadata {
    total_size_in_bytes: u64,
    latest_version: u64,
    first_version: u64,
}

/// InMemoryCache is a simple in-memory cache that stores the protobuf Transaction.
#[derive(Debug)]
pub struct InMemoryCache {
    /// Cache maps the cache key to the deserialized Transaction.
    cache: Arc<DashMap<u64, Arc<Transaction>>>,
    cache_metadata: Arc<RwLock<CacheMetadata>>,
    _cancellation_token_drop_guard: tokio_util::sync::DropGuard,
}

impl InMemoryCache {
    pub async fn new_with_redis_connection<C>(
        cache_config: InMemoryCacheConfig,
        conn: C,
        storage_format: StorageFormat,
    ) -> anyhow::Result<Self>
    where
        C: redis::aio::ConnectionLike + Send + Sync + Clone + 'static,
    {
        let cache = Arc::new(DashMap::new());
        let (in_memory_first_version, in_memory_latest_version, total_size_in_bytes) =
            warm_up_the_cache(conn.clone(), cache.clone(), storage_format).await?;
        tracing::info!(
            "In-memory cache is warmed up to version {}",
            in_memory_latest_version
        );
        let cancellation_token = tokio_util::sync::CancellationToken::new();
        let cache_metadata = Arc::new(RwLock::new(CacheMetadata {
            first_version: in_memory_first_version,
            total_size_in_bytes,
            latest_version: in_memory_latest_version,
        }));
        spawn_update_task(
            conn,
            cache.clone(),
            cache_metadata.clone(),
            storage_format,
            cancellation_token.clone(),
        );
        spawn_cleanup_task(
            cache_config.size_config.clone(),
            cache.clone(),
            cache_metadata.clone(),
            cancellation_token.clone(),
        );
        tracing::info!("In-memory cache is created");
        Ok(Self {
            cache,
            cache_metadata,
            _cancellation_token_drop_guard: cancellation_token.drop_guard(),
        })
    }

    pub async fn latest_version(&self) -> u64 {
        self.cache_metadata.read().await.latest_version
    }

    // This returns the transaction if it exists in the cache.
    // If requested version is not in the cache, it blocks until the version is available.
    // Otherwise, empty.
    pub async fn get_transactions(&self, starting_version: u64) -> Vec<Transaction> {
        let start_time = std::time::Instant::now();
        let (versions_to_fetch, in_memory_latest_version) = loop {
            let latest_version = self.latest_version().await;
            if starting_version >= latest_version {
                tokio::time::sleep(std::time::Duration::from_millis(
                    IN_MEMORY_CACHE_LOOKUP_RETRY_INTERVAL_MS,
                ))
                .await;
                continue;
            }
            // This is to avoid fetching too many transactions at once.
            let ending_version = std::cmp::min(
                latest_version,
                starting_version + MAX_FETCH_BATCH_SIZE as u64,
            );
            break (
                (starting_version..ending_version).collect::<Vec<u64>>(),
                latest_version,
            );
        };
        let lock_waiting_time = start_time.elapsed().as_secs_f64();
        let mut arc_transactions = Vec::new();
        for key in versions_to_fetch {
            if let Some(transaction) = self.cache.get(&key) {
                arc_transactions.push(transaction.clone());
            } else {
                break;
            }
        }

        let map_lookup_time = start_time.elapsed().as_secs_f64();
        // Actual clone.
        let res: Vec<Transaction> = arc_transactions
            .into_iter()
            .map(|t| t.as_ref().clone())
            .collect();
        let actual_copy_time = start_time.elapsed().as_secs_f64();
        tracing::info!(
            transactions_count = res.len(),
            starting_version,
            in_memory_latest_version,
            duration_in_seconds = start_time.elapsed().as_secs_f64(),
            lock_waiting_time,
            map_lookup_time,
            actual_copy_time,
            "In-memory cache lookup",
        );
        res
    }
}

/// Warm up the cache with the latest transactions.
async fn warm_up_the_cache<C>(
    conn: C,
    cache: Arc<DashMap<u64, Arc<Transaction>>>,
    storage_format: StorageFormat,
) -> anyhow::Result<(u64, u64, u64)>
where
    C: redis::aio::ConnectionLike + Send + Sync + Clone + 'static,
{
    let mut conn = conn.clone();
    let latest_version = get_config_by_key(&mut conn, "latest_version")
        .await
        .context("Failed to fetch the latest version from redis")?
        .context("Latest version doesn't exist in Redis")?;
    if latest_version == 0 {
        return Ok((0, 0, 0));
    }
    let versions_to_fetch: Vec<u64> =
        (latest_version.saturating_sub(WARM_UP_CACHE_ENTRIES)..latest_version).collect();
    let first_version = versions_to_fetch[0];
    let transactions = batch_get_transactions(&mut conn, versions_to_fetch, storage_format).await?;
    let total_size_in_bytes = transactions.iter().map(|t| t.encoded_len() as u64).sum();
    for transaction in transactions {
        cache.insert(transaction.version, Arc::new(transaction));
    }
    Ok((first_version, latest_version, total_size_in_bytes))
}

fn spawn_update_task<C>(
    conn: C,
    cache: Arc<DashMap<u64, Arc<Transaction>>>,
    cache_metadata: Arc<RwLock<CacheMetadata>>,
    storage_format: StorageFormat,
    cancellation_token: tokio_util::sync::CancellationToken,
) where
    C: redis::aio::ConnectionLike + Send + Sync + Clone + 'static,
{
    tokio::spawn(async move {
        let mut conn = conn.clone();
        loop {
            if cancellation_token.is_cancelled() {
                tracing::info!("In-memory cache update task is cancelled.");
                return;
            }
            let current_latest_version = get_config_by_key(&mut conn, "latest_version")
                .await
                .context("Failed to fetch the latest version from redis")
                .unwrap()
                .context("Latest version doesn't exist in Redis")
                .unwrap();
            let in_cache_latest_version = { cache_metadata.read().await.latest_version };
            if current_latest_version == in_cache_latest_version {
                tokio::time::sleep(std::time::Duration::from_millis(
                    IN_MEMORY_CACHE_LOOKUP_RETRY_INTERVAL_MS,
                ))
                .await;
                continue;
            }
            let end_version = std::cmp::min(
                current_latest_version,
                in_cache_latest_version + 10 * MAX_FETCH_BATCH_SIZE as u64,
            );
            let versions_to_fetch = (in_cache_latest_version..end_version).collect();
            let transactions = batch_get_transactions(&mut conn, versions_to_fetch, storage_format)
                .await
                .unwrap();
            // Ensure that transactions are ordered by version.
            let mut newly_added_bytes = 0;
            for (ind, transaction) in transactions.iter().enumerate() {
                if transaction.version != in_cache_latest_version + ind as u64 {
                    panic!("Transactions are not ordered by version");
                }
                newly_added_bytes += transaction.encoded_len() as u64;
            }
            for transaction in transactions {
                cache.insert(transaction.version, Arc::new(transaction));
            }
            let mut current_cache_metadata = { *cache_metadata.read().await };
            current_cache_metadata.latest_version = end_version;
            current_cache_metadata.total_size_in_bytes += newly_added_bytes;
            // Get the data available.
            {
                *cache_metadata.write().await = current_cache_metadata;
            }
        }
    });
}

fn spawn_cleanup_task(
    cache_size_config: InMemoryCacheSizeConfig,
    cache: Arc<DashMap<u64, Arc<Transaction>>>,
    cache_metadata: Arc<RwLock<CacheMetadata>>,
    cancellation_token: tokio_util::sync::CancellationToken,
) {
    tokio::spawn(async move {
        loop {
            if cancellation_token.is_cancelled() {
                tracing::info!("In-memory cache cleanup task is cancelled.");
                return;
            }
            let mut current_cache_metadata = { *cache_metadata.read().await };
            let should_evict = current_cache_metadata
                .total_size_in_bytes
                .saturating_sub(cache_size_config.cache_eviction_trigger_size_bytes)
                > 0;
            if !should_evict {
                tokio::time::sleep(std::time::Duration::from_millis(
                    IN_MEMORY_CACHE_GC_INTERVAL_MS,
                ))
                .await;
                continue;
            }
            let mut actual_bytes_removed = 0;
            let mut bytes_to_remove = current_cache_metadata
                .total_size_in_bytes
                .saturating_sub(cache_size_config.cache_target_size_bytes);
            while bytes_to_remove > 0 {
                let key_to_remove = current_cache_metadata.first_version;
                let (_k, v) = cache
                    .remove(&key_to_remove)
                    .expect("Failed to remove the key");
                bytes_to_remove = bytes_to_remove.saturating_sub(v.encoded_len() as u64);
                actual_bytes_removed += v.encoded_len() as u64;
                current_cache_metadata.first_version += 1;
            }
            current_cache_metadata.total_size_in_bytes -= actual_bytes_removed;
            *cache_metadata.write().await = current_cache_metadata;
        }
    });
}

// TODO: move the following functions to cache operator.
async fn get_config_by_key<C>(conn: &mut C, key: &str) -> anyhow::Result<Option<u64>>
where
    C: redis::aio::ConnectionLike + Send + Sync + Clone + 'static,
{
    let value = redis::cmd("GET").arg(key).query_async(conn).await?;
    Ok(value)
}

async fn batch_get_transactions<C>(
    conn: &mut C,
    versions: Vec<u64>,
    storage_format: StorageFormat,
) -> anyhow::Result<Vec<Transaction>>
where
    C: redis::aio::ConnectionLike + Send + Sync + Clone + 'static,
{
    let start_time = std::time::Instant::now();
    let keys: Vec<String> = versions
        .into_iter()
        .map(|version| CacheEntry::build_key(version, storage_format))
        .collect();
    let mut tasks: Vec<tokio::task::JoinHandle<anyhow::Result<Vec<Transaction>>>> = Vec::new();
    for chunk in &keys.into_iter().chunks(MAX_REDIS_FETCH_BATCH_SIZE) {
        let keys: Vec<String> = chunk.collect();
        let mut conn = conn.clone();
        tasks.push(tokio::spawn(async move {
            let values = conn.mget::<Vec<String>, Vec<Vec<u8>>>(keys).await?;
            // If any of the values are empty, we return an error.
            if values.iter().any(|v| v.is_empty()) {
                return Err(anyhow::anyhow!(format!(
                    "Failed to fetch all the keys; fetch size {}",
                    values.len()
                )));
            }
            let transactions = values
                .into_iter()
                .map(|v| {
                    let cache_entry = CacheEntry::new(v, storage_format);
                    cache_entry.into_transaction()
                })
                .collect();
            Ok(transactions)
        }));
    }
    let task_count = tasks.len();
    // join all.
    let results = futures::future::join_all(tasks).await;
    let fetching_duration = start_time.elapsed().as_secs_f64();
    let mut transactions = Vec::new();
    for result in results {
        transactions.extend(result??);
    }
    let total_size_in_bytes: u64 = transactions.iter().map(|t| t.encoded_len() as u64).sum();
    tracing::info!(
        fetching_duration,
        total_size_in_bytes,
        task_count,
        "In-memory batch get transactions"
    );
    anyhow::Result::Ok(transactions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis_test::{MockCmd, MockRedisConnection};

    fn generate_redis_value_bulk(
        starting_version: u64,
        storage_format: StorageFormat,
        size: usize,
    ) -> redis::Value {
        redis::Value::Bulk(
            (starting_version..starting_version + size as u64)
                .map(|e| {
                    let txn = Transaction {
                        version: e,
                        block_height: 1,
                        ..Default::default()
                    };
                    let cache_entry = CacheEntry::from_transaction(txn, storage_format);
                    redis::Value::Data(cache_entry.into_inner())
                })
                .collect(),
        )
    }

    fn generate_redis_key_bulk(
        starting_version: u64,
        storage_format: StorageFormat,
        size: usize,
    ) -> Vec<String> {
        (starting_version..starting_version + size as u64)
            .map(|e| CacheEntry::build_key(e, storage_format))
            .collect()
    }

    #[tokio::test]
    async fn test_in_memory_cache_with_zero_entries() {
        let mock_connection = MockRedisConnection::new(vec![MockCmd::new(
            redis::cmd("GET").arg("latest_version"),
            Ok(0),
        )]);
        let in_memory_cache = InMemoryCache::new_with_redis_connection(
            InMemoryCacheConfig::default(),
            mock_connection.clone(),
            StorageFormat::Base64UncompressedProto,
        )
        .await
        .unwrap();

        assert_eq!(in_memory_cache.latest_version().await, 0);
    }

    #[tokio::test]
    async fn test_in_memory_cache_with_one_entry() {
        let mock_connection = MockRedisConnection::new(vec![
            MockCmd::new(redis::cmd("GET").arg("latest_version"), Ok(1)),
            MockCmd::new(
                redis::cmd("MGET").arg(generate_redis_key_bulk(
                    0,
                    StorageFormat::Base64UncompressedProto,
                    1,
                )),
                Ok(generate_redis_value_bulk(
                    0,
                    StorageFormat::Base64UncompressedProto,
                    1,
                )),
            ),
        ]);
        let in_memory_cache = InMemoryCache::new_with_redis_connection(
            InMemoryCacheConfig::default(),
            mock_connection.clone(),
            StorageFormat::Base64UncompressedProto,
        )
        .await
        .unwrap();

        assert_eq!(in_memory_cache.latest_version().await, 1);
        let txns = in_memory_cache.get_transactions(0).await;
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].version, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_in_memory_cache_with_2_batches() {
        let mock_connection = MockRedisConnection::new(vec![
            MockCmd::new(redis::cmd("GET").arg("latest_version"), Ok(1)),
            MockCmd::new(
                redis::cmd("MGET").arg(generate_redis_key_bulk(
                    0,
                    StorageFormat::Base64UncompressedProto,
                    1,
                )),
                Ok(generate_redis_value_bulk(
                    0,
                    StorageFormat::Base64UncompressedProto,
                    1,
                )),
            ),
            MockCmd::new(redis::cmd("GET").arg("latest_version"), Ok(2)),
            MockCmd::new(
                redis::cmd("MGET").arg(generate_redis_key_bulk(
                    1,
                    StorageFormat::Base64UncompressedProto,
                    1,
                )),
                Ok(generate_redis_value_bulk(
                    1,
                    StorageFormat::Base64UncompressedProto,
                    1,
                )),
            ),
            MockCmd::new(redis::cmd("GET").arg("latest_version"), Ok(2)),
        ]);
        let in_memory_cache = InMemoryCache::new_with_redis_connection(
            InMemoryCacheConfig::default(),
            mock_connection.clone(),
            StorageFormat::Base64UncompressedProto,
        )
        .await
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        assert_eq!(in_memory_cache.latest_version().await, 2);
        let txns = in_memory_cache.get_transactions(1).await;
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].version, 1);
    }
}
