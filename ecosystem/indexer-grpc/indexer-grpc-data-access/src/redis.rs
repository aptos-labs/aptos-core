// Copyright © Aptos Foundation

use crate::{
    access_trait::{AccessMetadata, StorageReadError, StorageReadStatus, StorageTransactionRead},
    REDIS_CHAIN_ID, REDIS_ENDING_VERSION_EXCLUSIVE_KEY,
};
use anyhow::Context;
use aptos_indexer_grpc_utils::{
    storage::{CacheEntry, CacheEntryKey, StorageFormat},
    types::RedisUrl,
};
use aptos_protos::transaction::v1::Transaction;
use redis::{aio::ConnectionLike, AsyncCommands, ErrorKind};
use serde::{Deserialize, Serialize};

const REDIS_STORAGE_NAME: &str = "Redis";
const DEFAULT_REDIS_MGET_BATCH_SIZE: usize = 1000;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RedisClientConfig {
    // The source of the transactions.
    redis_address: RedisUrl,
    #[serde(default = "default_storage_format")]
    storage_format: StorageFormat,
}

fn default_storage_format() -> StorageFormat {
    StorageFormat::Base64UncompressedProto
}

pub type RedisClient = RedisClientInternal<redis::aio::ConnectionManager>;

impl RedisClient {
    pub async fn new(config: RedisClientConfig) -> anyhow::Result<Self> {
        let redis_client = redis::Client::open(config.redis_address.0.clone())
            .context("Failed to create Redis client.")?;
        let redis_connection = redis_client
            .get_tokio_connection_manager()
            .await
            .context("Failed to create Redis connection.")?;
        Ok(Self::new_with_connection(
            redis_connection,
            config.storage_format,
        ))
    }
}

#[derive(Clone)]
pub struct RedisClientInternal<C: ConnectionLike + Sync + Send + Clone> {
    // Redis Connection.
    pub redis_connection: C,
    pub storage_format: StorageFormat,
}

impl<C: ConnectionLike + Sync + Send + Clone> RedisClientInternal<C> {
    pub fn new_with_connection(redis_connection: C, storage_format: StorageFormat) -> Self {
        Self {
            redis_connection,
            storage_format,
        }
    }
}

impl From<redis::RedisError> for StorageReadError {
    fn from(err: redis::RedisError) -> Self {
        match err.kind() {
            // Fetch an entry that is not set yet.
            ErrorKind::TypeError => {
                StorageReadError::PermenantError(REDIS_STORAGE_NAME, anyhow::Error::new(err))
            },
            // Other errors are transient; let it retry.
            _ => StorageReadError::TransientError(REDIS_STORAGE_NAME, anyhow::Error::new(err)),
        }
    }
}

#[async_trait::async_trait]
impl<C: ConnectionLike + Sync + Send + Clone> StorageTransactionRead for RedisClientInternal<C> {
    async fn get_transactions(
        &self,
        batch_starting_version: u64,
        size_hint: Option<usize>,
    ) -> Result<StorageReadStatus, StorageReadError> {
        // Check the latest version of the cache.
        let mut conn = self.redis_connection.clone();
        let redis_ending_version_exclusive: u64 =
            conn.get(REDIS_ENDING_VERSION_EXCLUSIVE_KEY).await?;
        if batch_starting_version >= redis_ending_version_exclusive {
            return Ok(StorageReadStatus::NotAvailableYet);
        }

        let fetch_size = match size_hint {
            Some(size) => size,
            None => DEFAULT_REDIS_MGET_BATCH_SIZE,
        };
        let batch_ending_version_exclusive = std::cmp::min(
            batch_starting_version + fetch_size as u64,
            redis_ending_version_exclusive,
        );
        // Use MGET to fetch the transactions in batches.
        // let keys: Vec<u64> = (batch_starting_version..batch_ending_version_exclusive).collect();
        let keys: Vec<String> = (batch_starting_version..batch_ending_version_exclusive)
            .map(|version| CacheEntryKey::new(version, self.storage_format).to_string())
            .collect::<Vec<String>>();
        let result = conn.mget::<Vec<String>, Vec<Vec<u8>>>(keys).await;
        match result {
            Ok(serialized_transactions) => {
                // if any of the transactions are missing, return NotFound.
                if serialized_transactions
                    .iter()
                    .any(|serialized_transaction| serialized_transaction.is_empty())
                {
                    return Ok(StorageReadStatus::NotFound);
                }
                Ok(StorageReadStatus::Ok(
                    serialized_transactions
                        .into_iter()
                        .map(|serialized_transaction| {
                            let cache_entry = match self.storage_format {
                                StorageFormat::Base64UncompressedProto => {
                                    CacheEntry::Base64UncompressedProto(serialized_transaction)
                                },
                                StorageFormat::Bz2CompressedProto => {
                                    CacheEntry::Bz2CompressedProto(serialized_transaction)
                                },
                                StorageFormat::GzipCompressionProto => {
                                    CacheEntry::GzipCompressionProto(serialized_transaction)
                                },
                                _ => {
                                    panic!("Unsupported storage format: {:?}", self.storage_format)
                                },
                            };
                            let transaction: Transaction = cache_entry
                                .try_into()
                                .expect("Failed to deserialize transaction.");
                            transaction
                        })
                        .collect::<Vec<Transaction>>(),
                ))
            },
            Err(err) => {
                match err.kind() {
                    // If entries are evicted from the cache, Redis returns NIL, which is not String type.
                    // We treat this as NotFound.
                    ErrorKind::TypeError => Ok(StorageReadStatus::NotFound),
                    // Other errors are transient; let it retry.
                    _ => Err(StorageReadError::TransientError(
                        REDIS_STORAGE_NAME,
                        anyhow::Error::new(err),
                    )),
                }
            },
        }
    }

    async fn get_metadata(&self) -> Result<AccessMetadata, StorageReadError> {
        let mut conn = self.redis_connection.clone();
        let chain_id = conn.get(REDIS_CHAIN_ID).await?;
        let next_version = conn.get(REDIS_ENDING_VERSION_EXCLUSIVE_KEY).await?;
        Ok(AccessMetadata {
            chain_id,
            next_version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;
    use redis_test::{MockCmd, MockRedisConnection};

    #[tokio::test]
    async fn test_redis_metadata_fetch_success() {
        let mock_connection = MockRedisConnection::new(vec![
            MockCmd::new(redis::cmd("GET").arg(REDIS_CHAIN_ID), Ok(1)),
            MockCmd::new(
                redis::cmd("GET").arg(REDIS_ENDING_VERSION_EXCLUSIVE_KEY),
                Ok(1000),
            ),
        ]);
        let redis_client = RedisClientInternal::new_with_connection(
            mock_connection.clone(),
            StorageFormat::Base64UncompressedProto,
        );
        let metadata = redis_client.get_metadata().await.unwrap();
        assert_eq!(metadata.chain_id, 1);
        assert_eq!(metadata.next_version, 1000);
    }

    #[tokio::test]
    async fn test_redis_metadata_fetch_failure() {
        let mock_connection = MockRedisConnection::new(vec![MockCmd::new(
            redis::cmd("GET").arg(REDIS_CHAIN_ID),
            Ok(redis::Value::Nil),
        )]);
        let redis_client = RedisClientInternal::new_with_connection(
            mock_connection.clone(),
            StorageFormat::Base64UncompressedProto,
        );
        let metadata = redis_client.get_metadata().await;
        assert!(metadata.is_err());
        assert!(matches!(
            metadata.unwrap_err(),
            StorageReadError::PermenantError(REDIS_STORAGE_NAME, _)
        ));
    }

    #[tokio::test]
    async fn test_redis_transactions_fetch_success() {
        let transaction = Transaction {
            version: 42,
            ..Transaction::default()
        };
        let mut buf = Vec::new();
        transaction.encode(&mut buf).unwrap();
        let base64_encoded = base64::encode(buf);
        let values = redis::Value::Bulk(vec![redis::Value::Data(base64_encoded.into())]);
        let mock_connection = MockRedisConnection::new(vec![
            MockCmd::new(
                redis::cmd("GET").arg(REDIS_ENDING_VERSION_EXCLUSIVE_KEY),
                Ok(43),
            ),
            MockCmd::new(redis::cmd("MGET").arg("42"), Ok(values)),
        ]);
        let redis_client = RedisClientInternal::new_with_connection(
            mock_connection.clone(),
            StorageFormat::Base64UncompressedProto,
        );
        let transactions = redis_client.get_transactions(42, Some(1)).await;
        assert!(transactions.is_ok());
        let transactions = transactions.unwrap();
        assert_eq!(transactions, StorageReadStatus::Ok(vec![transaction]));
    }

    #[tokio::test]
    async fn test_redis_transactions_fetch_data_not_ready_yet() {
        let mock_connection = MockRedisConnection::new(vec![MockCmd::new(
            redis::cmd("GET").arg(REDIS_ENDING_VERSION_EXCLUSIVE_KEY),
            Ok(30),
        )]);
        let redis_client = RedisClientInternal::new_with_connection(
            mock_connection.clone(),
            StorageFormat::Base64UncompressedProto,
        );
        let transactions = redis_client.get_transactions(42, Some(1)).await;
        assert!(transactions.is_ok());
        let transactions = transactions.unwrap();
        assert_eq!(transactions, StorageReadStatus::NotAvailableYet);
    }

    #[tokio::test]
    async fn test_redis_transactions_fetch_data_not_found() {
        let transaction = Transaction {
            version: 42,
            ..Transaction::default()
        };
        let encoded = transaction.encode_to_vec();
        let base64_encoded = base64::encode(encoded).as_bytes().to_vec();

        let values =
            redis::Value::Bulk(vec![redis::Value::Nil, redis::Value::Data(base64_encoded)]);
        let mock_connection = MockRedisConnection::new(vec![
            MockCmd::new(
                redis::cmd("GET").arg(REDIS_ENDING_VERSION_EXCLUSIVE_KEY),
                Ok(43),
            ),
            MockCmd::new(redis::cmd("MGET").arg("41").arg("42"), Ok(values)),
        ]);
        let redis_client = RedisClientInternal::new_with_connection(
            mock_connection.clone(),
            StorageFormat::Base64UncompressedProto,
        );
        let transactions = redis_client.get_transactions(41, Some(2)).await;
        assert!(transactions.is_ok());
        let transactions = transactions.unwrap();
        assert_eq!(transactions, StorageReadStatus::NotFound);
    }
}
