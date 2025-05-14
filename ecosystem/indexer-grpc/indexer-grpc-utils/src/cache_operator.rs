// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    compression_util::{CacheEntry, StorageFormat, FILE_ENTRY_TRANSACTION_COUNT},
    counters::{log_grpc_step, IndexerGrpcStep},
};
use anyhow::{ensure, Context};
use aptos_protos::transaction::v1::Transaction;
use redis::{AsyncCommands, RedisResult};

// Configurations for cache.
// Cache entries that are present.
pub const CACHE_SIZE_ESTIMATION: u64 = 250_000_u64;

pub const MAX_CACHE_FETCH_SIZE: u64 = 1000_u64;

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
///   Returns 1 if the cache is updated but overlap detected.
///   Returns 2 if the cache is not updated and gap detected.
const CACHE_SCRIPT_UPDATE_LATEST_VERSION: &str = r#"
    local latest_version = redis.call("GET", KEYS[1])
    local num_of_versions = tonumber(ARGV[1])
    local current_version = tonumber(ARGV[2])
    if latest_version then
        if tonumber(latest_version) + num_of_versions < current_version then
            return 2
        elseif tonumber(latest_version) + num_of_versions == current_version then
            redis.call("SET", KEYS[1], current_version)
            return 0
        else
            redis.call("SET", KEYS[1], math.max(current_version, tonumber(latest_version)))
            return 1
        end
    else
        redis.call("SET", KEYS[1], ARGV[1])
        return 0
    end
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheBatchGetStatus {
    /// OK with batch of encoded transactions.
    Ok(Vec<Vec<u8>>),
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
pub enum CacheCoverageStatus {
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
        let _: () = self
            .conn
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

    /// Returns starting version and ending version.
    pub async fn get_latest_starting_and_ending_verisons(
        &mut self,
    ) -> anyhow::Result<Option<(u64, u64)>> {
        let latest_version = self.get_latest_version().await?;
        match latest_version {
            Some(version) => Ok(Some((
                version.saturating_sub(CACHE_SIZE_ESTIMATION),
                // Fix this: current latest version is exclusive.
                version.saturating_sub(1),
            ))),
            None => Ok(None),
        }
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
        let _: () = self
            .conn
            .set(FILE_STORE_LATEST_VERSION, latest_version)
            .await?;
        Ok(())
    }

    // Internal function to get the latest version from cache.
    pub async fn check_cache_coverage_status(
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
                FILE_ENTRY_TRANSACTION_COUNT,
            )))
        }
    }

    /// Fail if not all transactions requested are returned
    pub async fn batch_get_encoded_proto_data_with_length(
        &mut self,
        start_version: u64,
        transaction_count: u64,
    ) -> anyhow::Result<(Vec<Transaction>, f64, f64)> {
        let start_time = std::time::Instant::now();
        let versions = (start_version..start_version + transaction_count)
            .map(|e| CacheEntry::build_key(e, self.storage_format).to_string())
            .collect::<Vec<String>>();
        let encoded_transactions: Vec<Vec<u8>> = self
            .conn
            .mget(versions)
            .await
            .context("Failed to mget from Redis")?;
        let io_duration = start_time.elapsed().as_secs_f64();
        let start_time = std::time::Instant::now();
        let mut transactions = vec![];
        for encoded_transaction in encoded_transactions {
            let cache_entry: CacheEntry = CacheEntry::new(encoded_transaction, self.storage_format);
            let transaction = cache_entry.into_transaction();
            transactions.push(transaction);
        }
        ensure!(
            transactions.len() == transaction_count as usize,
            "Failed to get all transactions from cache."
        );
        let decoding_duration = start_time.elapsed().as_secs_f64();
        Ok((transactions, io_duration, decoding_duration))
    }

    pub async fn update_cache_transactions(
        &mut self,
        transactions: Vec<Transaction>,
    ) -> anyhow::Result<()> {
        let start_version = transactions.first().unwrap().version;
        let end_version = transactions.last().unwrap().version;
        let num_transactions = transactions.len();
        let start_txn_timestamp = transactions.first().unwrap().timestamp;
        let end_txn_timestamp = transactions.last().unwrap().timestamp;
        let mut size_in_bytes = 0;
        let mut redis_pipeline = redis::pipe();
        let start_time = std::time::Instant::now();
        for transaction in transactions {
            let version = transaction.version;
            let cache_key = CacheEntry::build_key(version, self.storage_format).to_string();
            let timestamp_in_seconds = transaction.timestamp.map_or(0, |t| t.seconds as u64);
            let cache_entry: CacheEntry =
                CacheEntry::from_transaction(transaction, self.storage_format);
            let bytes = cache_entry.into_inner();
            size_in_bytes += bytes.len();
            redis_pipeline
                .cmd("SET")
                .arg(cache_key)
                .arg(bytes)
                .arg("EX")
                .arg(get_ttl_in_seconds(timestamp_in_seconds))
                .ignore();
            // Actively evict the expired cache. This is to avoid using Redis
            // eviction policy, which is probabilistic-based and may evict the
            // cache that is still needed.
            if version >= CACHE_SIZE_EVICTION_LOWER_BOUND {
                let key = CacheEntry::build_key(
                    version - CACHE_SIZE_EVICTION_LOWER_BOUND,
                    self.storage_format,
                )
                .to_string();
                redis_pipeline.cmd("DEL").arg(key).ignore();
            }
        }
        // Note: this method is and should be only used by `cache_worker`.
        let service_type = "cache_worker";
        log_grpc_step(
            service_type,
            IndexerGrpcStep::CacheWorkerTxnEncoded,
            Some(start_version as i64),
            Some(end_version as i64),
            start_txn_timestamp.as_ref(),
            end_txn_timestamp.as_ref(),
            Some(start_time.elapsed().as_secs_f64()),
            Some(size_in_bytes),
            Some(num_transactions as i64),
            None,
        );

        let redis_result: RedisResult<()> =
            redis_pipeline.query_async::<_, _>(&mut self.conn).await;

        match redis_result {
            Ok(_) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    // Fetching from cache
    // Requested version x
    // Cache hit x +

    // TODO: Remove this
    pub async fn batch_get_encoded_proto_data(
        &mut self,
        start_version: u64,
    ) -> anyhow::Result<CacheBatchGetStatus> {
        let cache_coverage_status = self.check_cache_coverage_status(start_version).await;
        match cache_coverage_status {
            Ok(CacheCoverageStatus::CacheHit(v)) => {
                let versions = (start_version..start_version + v)
                    .map(|e| CacheEntry::build_key(e, self.storage_format))
                    .collect::<Vec<String>>();
                let encoded_transactions: Vec<Vec<u8>> = self.conn.mget(versions).await?;
                Ok(CacheBatchGetStatus::Ok(encoded_transactions))
            },
            Ok(CacheCoverageStatus::CacheEvicted) => Ok(CacheBatchGetStatus::EvictedFromCache),
            Ok(CacheCoverageStatus::DataNotReady) => Ok(CacheBatchGetStatus::NotReady),
            Err(err) => Err(err),
        }
    }

    // Update the latest version in cache.
    pub async fn update_cache_latest_version(
        &mut self,
        num_of_versions: u64,
        version: u64,
    ) -> anyhow::Result<()> {
        let script = redis::Script::new(CACHE_SCRIPT_UPDATE_LATEST_VERSION);
        tracing::debug!(
            num_of_versions = num_of_versions,
            version = version,
            "Updating latest version in cache."
        );
        match script
            .key(CACHE_KEY_LATEST_VERSION)
            .arg(num_of_versions)
            .arg(version)
            .invoke_async(&mut self.conn)
            .await
            .context("Redis latest version update failed.")?
        {
            2 => {
                tracing::error!(version=version, "Redis latest version update failed. The version is beyond the next expected version.");
                Err(anyhow::anyhow!("Version is not right."))
            },
            _ => Ok(()),
        }
    }

    pub async fn get_transactions_with_durations(
        &mut self,
        start_version: u64,
        transaction_count: u64,
    ) -> anyhow::Result<(Vec<Transaction>, f64, f64)> {
        let start_time = std::time::Instant::now();
        let versions = (start_version..start_version + transaction_count)
            .map(|e| CacheEntry::build_key(e, self.storage_format))
            .collect::<Vec<String>>();
        let encoded_transactions: Vec<Vec<u8>> = self
            .conn
            .mget(versions)
            .await
            .context("Failed to mget from Redis")?;
        let io_duration = start_time.elapsed().as_secs_f64();
        let start_time = std::time::Instant::now();
        let mut transactions = vec![];
        for encoded_transaction in encoded_transactions {
            let cache_entry: CacheEntry = CacheEntry::new(encoded_transaction, self.storage_format);
            let transaction = cache_entry.into_transaction();
            transactions.push(transaction);
        }
        ensure!(
            transactions.len() == transaction_count as usize,
            "Failed to get all transactions from cache."
        );
        let decoding_duration = start_time.elapsed().as_secs_f64();
        Ok((transactions, io_duration, decoding_duration))
    }

    /// Fail if not all transactions requested are returned
    pub async fn get_transactions(
        &mut self,
        start_version: u64,
        transaction_count: u64,
    ) -> anyhow::Result<Vec<Transaction>> {
        let (transactions, _, _) = self
            .get_transactions_with_durations(start_version, transaction_count)
            .await?;
        Ok(transactions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_protos::util::timestamp::Timestamp;
    use prost::Message;
    use redis_test::{MockCmd, MockRedisConnection};

    #[tokio::test]
    async fn cache_is_setup_if_empty() {
        // Key doesn't exists and SET_NX returns 1.
        let cmds = vec![MockCmd::new(
            redis::cmd("SET")
                .arg(CACHE_KEY_LATEST_VERSION)
                .arg(CACHE_DEFAULT_LATEST_VERSION_NUMBER)
                .arg("NX"),
            Ok("1"),
        )];
        let mock_connection = MockRedisConnection::new(cmds);
        let mut cache_operator: CacheOperator<MockRedisConnection> =
            CacheOperator::new(mock_connection, StorageFormat::Base64UncompressedProto);

        assert!(cache_operator.cache_setup_if_needed().await.unwrap());
    }

    #[tokio::test]
    async fn cache_is_setup_if_not_empty() {
        let cmds = vec![MockCmd::new(
            redis::cmd("SET")
                .arg(CACHE_KEY_LATEST_VERSION)
                .arg(CACHE_DEFAULT_LATEST_VERSION_NUMBER)
                .arg("NX"),
            Ok("0"),
        )];
        let mock_connection = MockRedisConnection::new(cmds);
        let mut cache_operator: CacheOperator<MockRedisConnection> =
            CacheOperator::new(mock_connection, StorageFormat::Base64UncompressedProto);

        assert!(!cache_operator.cache_setup_if_needed().await.unwrap());
    }
    // Cache coverage status tests.
    #[tokio::test]
    async fn cache_coverage_status_is_not_ready() {
        let cmds = vec![MockCmd::new(
            redis::cmd("GET").arg(CACHE_KEY_LATEST_VERSION),
            Ok("12"),
        )];
        let mock_connection = MockRedisConnection::new(cmds);
        let mut cache_operator: CacheOperator<MockRedisConnection> =
            CacheOperator::new(mock_connection, StorageFormat::Base64UncompressedProto);

        assert_eq!(cache_operator.get_latest_version().await.unwrap(), Some(12));
    }

    // Cache chain id tests.
    #[tokio::test]
    async fn cache_chain_id_ok() {
        let cmds = vec![MockCmd::new(
            redis::cmd("GET").arg(CACHE_KEY_CHAIN_ID),
            Ok("123"),
        )];
        let mock_connection = MockRedisConnection::new(cmds);
        let mut cache_operator: CacheOperator<MockRedisConnection> =
            CacheOperator::new(mock_connection, StorageFormat::Base64UncompressedProto);

        assert_eq!(cache_operator.get_chain_id().await.unwrap(), Some(123));
    }

    // Cache latest version tests.
    #[tokio::test]
    async fn cache_latest_version_ok() {
        let version = 123_u64;
        let cmds = vec![MockCmd::new(
            redis::cmd("GET").arg(CACHE_KEY_LATEST_VERSION),
            Ok(version.to_string()),
        )];
        let mock_connection = MockRedisConnection::new(cmds);
        let mut cache_operator: CacheOperator<MockRedisConnection> =
            CacheOperator::new(mock_connection, StorageFormat::Base64UncompressedProto);

        assert_eq!(
            cache_operator.get_latest_version().await.unwrap(),
            Some(version)
        );
    }

    // Cache update cache transactions tests.
    #[tokio::test]
    async fn cache_update_cache_transactions_ok() {
        let transactions = vec![Transaction {
            version: 1,
            timestamp: Some(Timestamp {
                seconds: 1,
                nanos: 0,
            }),
            ..Default::default()
        }];
        let mut buf = vec![];
        let key = "1";
        transactions[0].encode(&mut buf).unwrap();
        let encoded_proto_data = base64::encode(&buf);
        let cmds = vec![MockCmd::new(
            redis::cmd("SET")
                .arg(key)
                .arg(encoded_proto_data.clone())
                .arg("EX")
                .arg(get_ttl_in_seconds(1)),
            Ok("ok"),
        )];
        let mock_connection = MockRedisConnection::new(cmds);
        let mut cache_operator: CacheOperator<MockRedisConnection> =
            CacheOperator::new(mock_connection, StorageFormat::Base64UncompressedProto);
        assert!(cache_operator
            .update_cache_transactions(transactions)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn cache_update_cache_transactions_with_large_version_ok() {
        let version = CACHE_SIZE_EVICTION_LOWER_BOUND + 100;
        let transactions = vec![Transaction {
            version,
            timestamp: Some(Timestamp {
                seconds: 1,
                nanos: 0,
            }),
            ..Default::default()
        }];
        let mut buf = vec![];
        transactions[0].encode(&mut buf).unwrap();
        let encoded_proto_data = base64::encode(&buf);
        let mut redis_pipeline = redis::pipe();
        redis_pipeline
            .cmd("SET")
            .arg(version.to_string())
            .arg(encoded_proto_data)
            .arg("EX")
            .arg(get_ttl_in_seconds(1));
        redis_pipeline
            .cmd("DEL")
            .arg(version - CACHE_SIZE_EVICTION_LOWER_BOUND);
        let cmds = vec![MockCmd::new(redis_pipeline, Ok("ok"))];
        let mock_connection = MockRedisConnection::new(cmds);
        let mut cache_operator: CacheOperator<MockRedisConnection> =
            CacheOperator::new(mock_connection, StorageFormat::Base64UncompressedProto);
        let res = cache_operator.update_cache_transactions(transactions).await;
        println!("{:?}", res);
        assert!(res.is_ok());
    }
}
