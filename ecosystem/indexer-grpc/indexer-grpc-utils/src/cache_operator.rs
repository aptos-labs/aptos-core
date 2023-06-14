// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::constants::BLOB_STORAGE_SIZE;
use anyhow::Context;
use redis::{AsyncCommands, RedisError, RedisResult};

// Configurations for cache.
// The cache size is estimated to be 3M transactions.
// For 3M transactions, the cache size is about 25GB.
// At TPS 20k, it takes about 2.5 minutes to fill up the cache.
const CACHE_SIZE_ESTIMATION: u64 = 3_000_000_u64;

// Hard limit for cache lower bound. Only used for active eviction.
// Cache worker actively evicts the cache entries if the cache entry version is
// lower than the latest version - CACHE_SIZE_EVICTION_LOWER_BOUND.
// The gap between CACHE_SIZE_ESTIMATION and this is to give buffer since
// reading latest version and actual data not atomic(two operations).
const CACHE_SIZE_EVICTION_LOWER_BOUND: u64 = 4_000_000_u64;

// Keys for cache.
const CACHE_KEY_LATEST_VERSION: &str = "latest_version";
const CACHE_KEY_CHAIN_ID: &str = "chain_id";
// 9999-12-31 23:59:59. UTC.
const BASE_EXPIRATION_EPOCH_TIME_IN_SECONDS: u64 = 253_402_300_799;

// Default values for cache.
const CACHE_DEFAULT_LATEST_VERSION_NUMBER: &str = "0";

// Returns 1 if the chain id is updated or verified. Otherwise(chain id not match), returns 0.
// TODO(larry): add a test for this script.
const CACHE_SCRIPT_UPDATE_OR_VERIFY_CHAIN_ID: &str = r#"
    local chain_id = redis.call("GET", KEYS[1])
    if chain_id then
        if chain_id == ARGV[1] then
            return 1
        else
            return 0
        end
    else
        redis.call("SET", KEYS[1], ARGV[1])
        return 1
    end
"#;

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
    Ok(Vec<String>),
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
pub struct CacheOperator<T: redis::aio::ConnectionLike + Send> {
    conn: T,
}

impl<T: redis::aio::ConnectionLike + Send> CacheOperator<T> {
    pub fn new(conn: T) -> Self {
        Self { conn }
    }

    // Set up the cache if needed.
    pub async fn cache_setup_if_needed(&mut self) -> bool {
        let version_inserted: bool = redis::cmd("SET")
            .arg(CACHE_KEY_LATEST_VERSION)
            .arg(CACHE_DEFAULT_LATEST_VERSION_NUMBER)
            .arg("NX")
            .query_async(&mut self.conn)
            .await
            .expect("Redis latest_version check failed.");
        if version_inserted {
            tracing::info!(
                initialized_latest_version = CACHE_DEFAULT_LATEST_VERSION_NUMBER,
                "Cache latest version is initialized."
            );
        }
        version_inserted
    }

    // Update the chain id in cache if missing; otherwise, verify the chain id.
    // It's a fatal error if the chain id is not correct.
    pub async fn update_or_verify_chain_id(&mut self, chain_id: u64) -> anyhow::Result<()> {
        let script = redis::Script::new(CACHE_SCRIPT_UPDATE_OR_VERIFY_CHAIN_ID);
        let result: u8 = script
            .key(CACHE_KEY_CHAIN_ID)
            .arg(chain_id)
            .invoke_async(&mut self.conn)
            .await
            .expect("Redis chain id update/verification failed.");
        if result != 1 {
            anyhow::bail!("Chain id is not correct.");
        }
        Ok(())
    }

    // Downstream system can infer the chain id from cache.
    pub async fn get_chain_id(&mut self) -> anyhow::Result<u64> {
        let chain_id: u64 = match self.conn.get::<&str, String>(CACHE_KEY_CHAIN_ID).await {
            Ok(v) => v
                .parse::<u64>()
                .with_context(|| format!("Redis key {} is not a number.", CACHE_KEY_CHAIN_ID))?,
            Err(err) => return Err(err.into()),
        };
        Ok(chain_id)
    }

    pub async fn get_latest_version(&mut self) -> anyhow::Result<u64> {
        let chain_id: u64 = match self
            .conn
            .get::<&str, String>(CACHE_KEY_LATEST_VERSION)
            .await
        {
            Ok(v) => v.parse::<u64>().with_context(|| {
                format!("Redis key {} is not a number.", CACHE_KEY_LATEST_VERSION)
            })?,
            Err(err) => return Err(err.into()),
        };
        Ok(chain_id)
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
            Ok(CacheCoverageStatus::CacheHit(std::cmp::min(
                latest_version - requested_version,
                BLOB_STORAGE_SIZE as u64,
            )))
        }
    }

    pub async fn update_cache_transactions(
        &mut self,
        transactions: Vec<(u64, String, u64)>,
    ) -> anyhow::Result<()> {
        let mut redis_pipeline = redis::pipe();
        for (version, encoded_proto_data, timestamp_in_seconds) in transactions {
            redis_pipeline
                .cmd("SET")
                .arg(version)
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
            Ok(_) => Ok(()),
            Err(err) => Err(err.into()),
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
            .expect("Redis latest version update failed.")
        {
            2 => {
                tracing::error!(version=version, "Redis latest version update failed. The version is beyond the next expected version.");
                panic!("version is not right.");
            },
            _ => Ok(()),
        }
    }

    pub async fn batch_get_encoded_proto_data(
        &mut self,
        start_version: u64,
    ) -> anyhow::Result<CacheBatchGetStatus> {
        let cache_coverage_status = self.check_cache_coverage_status(start_version).await;
        match cache_coverage_status {
            Ok(CacheCoverageStatus::CacheHit(v)) => {
                let versions = (start_version..start_version + v)
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>();
                let encoded_transactions: Result<Vec<String>, RedisError> =
                    self.conn.mget(versions).await;
                match encoded_transactions {
                    Ok(v) => Ok(CacheBatchGetStatus::Ok(v)),
                    Err(err) => Err(err.into()),
                }
            },
            Ok(CacheCoverageStatus::CacheEvicted) => Ok(CacheBatchGetStatus::EvictedFromCache),
            Ok(CacheCoverageStatus::DataNotReady) => Ok(CacheBatchGetStatus::NotReady),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            CacheOperator::new(mock_connection);

        assert!(cache_operator.cache_setup_if_needed().await);
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
            CacheOperator::new(mock_connection);

        assert!(!cache_operator.cache_setup_if_needed().await);
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
            CacheOperator::new(mock_connection);

        assert_eq!(
            cache_operator
                .check_cache_coverage_status(123)
                .await
                .unwrap(),
            CacheCoverageStatus::DataNotReady
        );
    }

    #[tokio::test]
    async fn cache_coverage_status_is_evicted() {
        let cmds = vec![MockCmd::new(
            redis::cmd("GET").arg(CACHE_KEY_LATEST_VERSION),
            Ok("120000000"),
        )];
        let mock_connection = MockRedisConnection::new(cmds);
        let mut cache_operator: CacheOperator<MockRedisConnection> =
            CacheOperator::new(mock_connection);

        assert_eq!(
            cache_operator.check_cache_coverage_status(1).await.unwrap(),
            CacheCoverageStatus::CacheEvicted
        );
    }

    #[tokio::test]
    async fn cache_coverage_status_cache_hit() {
        let cmds = vec![MockCmd::new(
            redis::cmd("GET").arg(CACHE_KEY_LATEST_VERSION),
            Ok("123"),
        )];
        let mock_connection = MockRedisConnection::new(cmds);
        let mut cache_operator: CacheOperator<MockRedisConnection> =
            CacheOperator::new(mock_connection);

        // Transactions are 100..123, thus 23 transactions are cached.
        assert_eq!(
            cache_operator
                .check_cache_coverage_status(100)
                .await
                .unwrap(),
            CacheCoverageStatus::CacheHit(23)
        );
    }

    #[tokio::test]
    async fn cache_coverage_status_cache_hit_with_full_batch() {
        let cmds = vec![MockCmd::new(
            redis::cmd("GET").arg(CACHE_KEY_LATEST_VERSION),
            Ok("12300"),
        )];
        let mock_connection = MockRedisConnection::new(cmds);
        let mut cache_operator: CacheOperator<MockRedisConnection> =
            CacheOperator::new(mock_connection);

        assert_eq!(
            cache_operator
                .check_cache_coverage_status(1000)
                .await
                .unwrap(),
            CacheCoverageStatus::CacheHit(1000)
        );
    }

    // Cache batch get status tests.
    #[tokio::test]
    async fn cache_batch_get_status_hit_the_head() {
        let bulck_value = redis::Value::Bulk(vec![
            redis::Value::Data("t0".as_bytes().to_vec()),
            redis::Value::Data("t1".as_bytes().to_vec()),
            redis::Value::Data("t2".as_bytes().to_vec()),
        ]);
        let cmds = vec![
            MockCmd::new(redis::cmd("GET").arg(CACHE_KEY_LATEST_VERSION), Ok("3")),
            MockCmd::new(
                redis::cmd("MGET").arg("0").arg("1").arg("2"),
                Ok(bulck_value),
            ),
        ];
        let mock_connection = MockRedisConnection::new(cmds);
        let mut cache_operator: CacheOperator<MockRedisConnection> =
            CacheOperator::new(mock_connection);

        assert_eq!(
            cache_operator
                .batch_get_encoded_proto_data(0)
                .await
                .unwrap(),
            CacheBatchGetStatus::Ok(vec!["t0".to_string(), "t1".to_string(), "t2".to_string()])
        );
    }

    #[tokio::test]
    async fn cache_batch_get_status_ok() {
        let bulck_value = redis::Value::Bulk(
            (1..1001)
                .map(|e| redis::Value::Data(format!("t{}", e).as_bytes().to_vec()))
                .collect(),
        );
        let keys = (1..1001).map(|e| e.to_string()).collect::<Vec<String>>();
        let cmds = vec![
            MockCmd::new(redis::cmd("GET").arg(CACHE_KEY_LATEST_VERSION), Ok("1003")),
            MockCmd::new(redis::cmd("MGET").arg(keys), Ok(bulck_value)),
        ];
        let mock_connection = MockRedisConnection::new(cmds);
        let mut cache_operator: CacheOperator<MockRedisConnection> =
            CacheOperator::new(mock_connection);

        assert_eq!(
            cache_operator
                .batch_get_encoded_proto_data(1)
                .await
                .unwrap(),
            CacheBatchGetStatus::Ok((1..1001).map(|e| format!("t{}", e)).collect())
        );
    }

    #[tokio::test]
    async fn cache_batch_get_status_cache_evicted() {
        let cmds = vec![MockCmd::new(
            redis::cmd("GET").arg(CACHE_KEY_LATEST_VERSION),
            Ok("100000000"),
        )];
        let mock_connection = MockRedisConnection::new(cmds);
        let mut cache_operator: CacheOperator<MockRedisConnection> =
            CacheOperator::new(mock_connection);

        assert_eq!(
            cache_operator
                .batch_get_encoded_proto_data(1)
                .await
                .unwrap(),
            CacheBatchGetStatus::EvictedFromCache
        );
    }

    #[tokio::test]
    async fn cache_batch_get_status_cache_not_ready() {
        let cmds = vec![MockCmd::new(
            redis::cmd("GET").arg(CACHE_KEY_LATEST_VERSION),
            Ok("1"),
        )];
        let mock_connection = MockRedisConnection::new(cmds);
        let mut cache_operator: CacheOperator<MockRedisConnection> =
            CacheOperator::new(mock_connection);

        assert_eq!(
            cache_operator
                .batch_get_encoded_proto_data(100_000_000)
                .await
                .unwrap(),
            CacheBatchGetStatus::NotReady
        );
    }

    // TODO:Cache update tests.

    // Cache chain id tests.
    #[tokio::test]
    async fn cache_chain_id_ok() {
        let cmds = vec![MockCmd::new(
            redis::cmd("GET").arg(CACHE_KEY_CHAIN_ID),
            Ok("123"),
        )];
        let mock_connection = MockRedisConnection::new(cmds);
        let mut cache_operator: CacheOperator<MockRedisConnection> =
            CacheOperator::new(mock_connection);

        assert_eq!(cache_operator.get_chain_id().await.unwrap(), 123);
    }
}
