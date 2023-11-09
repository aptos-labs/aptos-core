// Copyright © Aptos Foundation

use crate::{access_trait::AccessMetadata, REDIS_CHAIN_ID, REDIS_ENDING_VERSION_EXCLUSIVE_KEY};
use anyhow::Context;
use aptos_protos::transaction::v1::Transaction;
use dashmap::DashMap;
use prost::Message;
use redis::AsyncCommands;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

// Shared data between reads and writes.
type ThreadSafeAccessMetadata = Arc<RwLock<Option<AccessMetadata>>>;
type ThreadSafeInMemoryStorageStatus = Arc<RwLock<anyhow::Result<()>>>;
// Note: Arc<Transaction> is to avoid copying the transaction when operating on the map.
type TransactionMap = Arc<DashMap<u64, Arc<Transaction>>>;

// Capacity of the in-memory storage.
pub const IN_MEMORY_STORAGE_SIZE_SOFT_LIMIT: usize = 100_000;
// Capacity of the in-memory storage.
const IN_MEMORY_STORAGE_SIZE_HARD_LIMIT: usize = 120_000;
// Redis fetch task interval in milliseconds.
const REDIS_FETCH_TASK_INTERVAL_IN_MILLIS: u64 = 10;
// Redis fetch MGET batch size.
const REDIS_FETCH_MGET_BATCH_SIZE: usize = 1000;

// InMemoryStorage is the in-memory storage for transactions.
pub struct InMemoryStorageInternal {
    pub transactions_map: TransactionMap,
    pub metadata: ThreadSafeAccessMetadata,
    pub storage_status: ThreadSafeInMemoryStorageStatus,
    _cancellation_token_drop_guard: tokio_util::sync::DropGuard,
}

impl InMemoryStorageInternal {
    async fn new_with_connection<C>(
        redis_connection: C,
        transaction_map_size: Option<usize>,
    ) -> anyhow::Result<Self>
    where
        C: redis::aio::ConnectionLike + Send + Sync + Clone + 'static,
    {
        let redis_connection = Arc::new(redis_connection);
        let transactions_map = Arc::new(DashMap::new());
        let transactions_map_clone = transactions_map.clone();
        let metadata = Arc::new(RwLock::new(None));
        let metadata_clone = metadata.clone();
        let cancellation_token = tokio_util::sync::CancellationToken::new();
        let cancellation_token_clone = cancellation_token.clone();
        let storage_status = Arc::new(RwLock::new(Ok(())));
        let storage_status_clone = storage_status.clone();
        tokio::task::spawn(async move {
            let result = redis_fetch_task(
                redis_connection,
                transactions_map_clone,
                metadata_clone,
                cancellation_token_clone,
                transaction_map_size,
            )
            .await;
            let mut storage_status = storage_status_clone.write().unwrap();
            *storage_status = result;
        });
        Ok(Self {
            transactions_map,
            metadata,
            _cancellation_token_drop_guard: cancellation_token.drop_guard(),
            storage_status,
        })
    }

    pub async fn new(redis_address: String) -> anyhow::Result<Self> {
        let redis_client =
            redis::Client::open(redis_address).context("Failed to open Redis client.")?;
        let redis_connection = redis_client
            .get_tokio_connection_manager()
            .await
            .context("Failed to get Redis connection.")?;
        Self::new_with_connection(redis_connection, None).await
    }
}

/// redis_fetch_task fetches the transactions from Redis and updates the in-memory storage.
/// It's expected to be run in a separate thread.
async fn redis_fetch_task<C>(
    redis_connection: Arc<C>,
    transactions_map: Arc<DashMap<u64, Arc<Transaction>>>,
    metadata: ThreadSafeAccessMetadata,
    cancellation_token: tokio_util::sync::CancellationToken,
    transaction_map_size: Option<usize>,
) -> anyhow::Result<()>
where
    C: redis::aio::ConnectionLike + Send + Sync + Clone + 'static,
{
    let current_connection = redis_connection.clone();
    loop {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                return Ok(());
            },
            _ = tokio::time::sleep(Duration::from_millis(REDIS_FETCH_TASK_INTERVAL_IN_MILLIS)) => {
                // Continue.
            },
        }
        let start_time = std::time::Instant::now();
        let mut conn = current_connection.as_ref().clone();
        let redis_chain_id: u64 = conn
            .get(REDIS_CHAIN_ID)
            .await
            .context("Failed to get the redis id")?;
        let redis_ending_version_exclusive: u64 = conn
            .get(REDIS_ENDING_VERSION_EXCLUSIVE_KEY)
            .await
            .context("Failed to get the ending version")?;
        // The new metadata to be updated.
        let new_metadata = AccessMetadata {
            chain_id: redis_chain_id,
            next_version: redis_ending_version_exclusive,
        };

        let transactions_map_size_hard_limit =
            transaction_map_size.unwrap_or(IN_MEMORY_STORAGE_SIZE_HARD_LIMIT);
        // 1. Determine the fetch size based on old metadata.
        let redis_fetch_size = match *metadata.read().unwrap() {
            Some(ref current_metadata) => {
                anyhow::ensure!(
                    current_metadata.chain_id == redis_chain_id,
                    "Chain ID mismatch."
                );
                redis_ending_version_exclusive.saturating_sub(current_metadata.next_version)
                    as usize
            },
            None => std::cmp::min(
                transactions_map_size_hard_limit,
                redis_ending_version_exclusive as usize,
            ),
        };
        // 2. Use MGET to fetch the transactions in batches.
        let starting_version = redis_ending_version_exclusive - redis_fetch_size as u64;
        let ending_version = redis_ending_version_exclusive;
        // Order doesn't matter here; it'll be available in the map until metadata is updated.
        let keys_batches: Vec<Vec<String>> = (starting_version..ending_version)
            .map(|version| version.to_string())
            .collect::<Vec<String>>()
            .chunks(REDIS_FETCH_MGET_BATCH_SIZE)
            .map(|x| x.to_vec())
            .collect();
        for keys in keys_batches {
            let redis_transactions: Vec<String> = conn
                .mget(keys)
                .await
                .context("Failed to MGET from redis.")
                .expect("lskajdlfkjlaj");
            let transactions: Vec<Arc<Transaction>> = redis_transactions
                .into_iter()
                .map(|serialized_transaction| {
                    // TODO: leverage FROM to do conversion.
                    let serialized_transaction = base64::decode(serialized_transaction.as_bytes())
                        .expect("Failed to decode base64.");
                    let transaction = Transaction::decode(serialized_transaction.as_slice())
                        .expect("Failed to decode transaction protobuf from Redis.");
                    Arc::new(transaction)
                })
                .collect();
            for transaction in transactions {
                transactions_map.insert(transaction.version, transaction);
            }
        }
        // 3. Update the metadata.
        {
            let mut current_metadata = metadata.write().unwrap();
            *current_metadata = Some(new_metadata.clone());
        }
        if redis_fetch_size == 0 {
            tracing::info!("Redis is not ready for current fetch. Wait.");
            continue;
        }
        // Garbage collection. Note, this is *not a thread safe* operation; readers should
        // return NOT_FOUND if the version is not found.
        let current_size = transactions_map.len();
        let lowest_version = new_metadata.next_version - current_size as u64;
        let count_of_transactions_to_remove =
            current_size.saturating_sub(transactions_map_size_hard_limit);
        (lowest_version..lowest_version + count_of_transactions_to_remove as u64).for_each(
            |version| {
                transactions_map.remove(&version);
            },
        );
        tracing::info!(
            redis_fetch_size = redis_fetch_size,
            time_spent_in_seconds = start_time.elapsed().as_secs_f64(),
            fetch_starting_version = new_metadata.next_version - redis_fetch_size as u64,
            fetch_ending_version_inclusive = new_metadata.next_version - 1,
            "Fetching transactions from Redis."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis_test::{MockCmd, MockRedisConnection};

    fn generate_redis_value_bulk(starting_version: u64, size: usize) -> redis::Value {
        redis::Value::Bulk(
            (starting_version..starting_version + size as u64)
                .map(|e| {
                    let txn = Transaction {
                        version: e,
                        ..Default::default()
                    };
                    let mut txn_buf = Vec::new();
                    txn.encode(&mut txn_buf).unwrap();
                    let encoded = base64::encode(txn_buf);
                    redis::Value::Data(encoded.as_bytes().to_vec())
                })
                .collect(),
        )
    }
    // This test is to start the in-memory storage with a empty Redis.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_redis_fetch_fresh() {
        let mock_connection = MockRedisConnection::new(vec![
            MockCmd::new(redis::cmd("GET").arg(REDIS_CHAIN_ID), Ok(1)),
            MockCmd::new(
                redis::cmd("GET").arg(REDIS_ENDING_VERSION_EXCLUSIVE_KEY),
                Ok(0),
            ),
        ]);
        let in_memory_storage = InMemoryStorageInternal::new_with_connection(mock_connection, None)
            .await
            .unwrap();
        // Wait for the fetch task to finish.
        tokio::time::sleep(std::time::Duration::from_millis(
            REDIS_FETCH_TASK_INTERVAL_IN_MILLIS * 2,
        ))
        .await;
        {
            let metadata = in_memory_storage.metadata.read().unwrap();
            assert_eq!(metadata.as_ref().unwrap().chain_id, 1);
            assert_eq!(metadata.as_ref().unwrap().next_version, 0);
        }
    }

    // This test is to start the in-memory storage with 1001 transactions in Redis.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_redis_fetch() {
        let first_batch = generate_redis_value_bulk(0, 1000);
        let second_batch = generate_redis_value_bulk(1000, 1);
        let keys = (0..1000)
            .map(|version| version.to_string())
            .collect::<Vec<String>>();
        let cmds = vec![
            MockCmd::new(redis::cmd("GET").arg(REDIS_CHAIN_ID), Ok(1)),
            MockCmd::new(
                redis::cmd("GET").arg(REDIS_ENDING_VERSION_EXCLUSIVE_KEY),
                Ok(1001),
            ),
            MockCmd::new(redis::cmd("MGET").arg::<Vec<String>>(keys), Ok(first_batch)),
            MockCmd::new(
                redis::cmd("MGET").arg::<Vec<String>>(vec!["1000".to_string()]),
                Ok(second_batch),
            ),
        ];
        let mock_connection = MockRedisConnection::new(cmds);
        let in_memory_storage = InMemoryStorageInternal::new_with_connection(mock_connection, None)
            .await
            .unwrap();
        // Wait for the fetch task to finish.
        tokio::time::sleep(std::time::Duration::from_millis(
            REDIS_FETCH_TASK_INTERVAL_IN_MILLIS * 10,
        ))
        .await;
        {
            let metadata = in_memory_storage.metadata.read().unwrap();
            assert_eq!(metadata.as_ref().unwrap().chain_id, 1);
            assert_eq!(metadata.as_ref().unwrap().next_version, 1001);
        }

        assert_eq!(in_memory_storage.transactions_map.len(), 1001);
    }

    // This test is to start the in-memory storage with 1000 transactions in Redis first
    // and then 2000 transactions for the second batch.
    // In-memory storage has size 500.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_redis_fetch_with_eviction() {
        let first_batch = generate_redis_value_bulk(0, 1000);
        let second_batch = generate_redis_value_bulk(1000, 1000);
        // Filling the empty in-memory storage.
        let keys = (500..1000)
            .map(|version| version.to_string())
            .collect::<Vec<String>>();
        let second_keys = (1000..2000)
            .map(|version| version.to_string())
            .collect::<Vec<String>>();
        let cmds = vec![
            MockCmd::new(redis::cmd("GET").arg(REDIS_CHAIN_ID), Ok(1)),
            MockCmd::new(
                redis::cmd("GET").arg(REDIS_ENDING_VERSION_EXCLUSIVE_KEY),
                Ok(1000),
            ),
            MockCmd::new(redis::cmd("MGET").arg::<Vec<String>>(keys), Ok(first_batch)),
            // Redis storage moves forward.
            MockCmd::new(redis::cmd("GET").arg(REDIS_CHAIN_ID), Ok(1)),
            MockCmd::new(
                redis::cmd("GET").arg(REDIS_ENDING_VERSION_EXCLUSIVE_KEY),
                Ok(2000),
            ),
            MockCmd::new(
                redis::cmd("MGET").arg::<Vec<String>>(second_keys),
                Ok(second_batch),
            ),
        ];
        let mock_connection = MockRedisConnection::new(cmds);
        let in_memory_storage =
            InMemoryStorageInternal::new_with_connection(mock_connection, Some(500))
                .await
                .unwrap();
        // Wait for the fetch task to finish.
        tokio::time::sleep(std::time::Duration::from_millis(
            REDIS_FETCH_TASK_INTERVAL_IN_MILLIS * 10,
        ))
        .await;
        {
            let metadata = in_memory_storage.metadata.read().unwrap();
            assert_eq!(metadata.as_ref().unwrap().chain_id, 1);
            assert_eq!(metadata.as_ref().unwrap().next_version, 2000);
        }

        assert_eq!(in_memory_storage.transactions_map.len(), 500);
    }
}
