// Copyright © Aptos Foundation

// This benchmark is used to test the performance of in-memory cache.
// Key performance requirements:
//   read speed
//      * 20k reads per second for small load -  2 KB per read for 10 clients.  - TPS.
//      * 500 reads per second for large load - 10 MB per read for 10 clients   - memory-(de)allocation.
//      * 100 reads per second for small load -  2 KB per read for 1000 clients - contended reads.

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use crate::in_memory_cache::InMemoryCache;
    use crate::in_memory_cache::MAX_REDIS_FETCH_BATCH_SIZE;
    use crate::in_memory_cache::WARM_UP_CACHE_ENTRIES;
    use crate::compression_util::{CacheEntry, StorageFormat};
    use aptos_protos::transaction::v1::Transaction;
    use aptos_protos::transaction::v1::TransactionInfo;
    use redis_test::*;
    use lazy_static::lazy_static;
    use once_cell::sync::OnceCell;
    use redis_test::MockCmd;
    use futures::stream::StreamExt;

    lazy_static! {
        static ref TRANSACTION_CONTENT: OnceCell<Vec<u8>> = OnceCell::new();
    }
    // Create a transaction with a given version and size.
    // This involves memory allocation.
    fn generate_transaction_bytes(version:u64, size: usize) -> Vec<u8> {
        let txn = Transaction {
            version,
            info: Some(TransactionInfo {
                hash: TRANSACTION_CONTENT.get_or_init(|| vec![1; size]).clone(),
                ..TransactionInfo::default()
            }),
            ..Transaction::default()
        };
        let storage_format = StorageFormat::GzipCompressedProto;
        let cache_entry = CacheEntry::from_transaction(txn, storage_format);
        cache_entry.into_inner()
    }

    fn mock_get_latest_version(version: i64) -> MockCmd {
        MockCmd::new(
            redis::cmd("GET").arg("latest_version"),
            Ok(version),
        )
    }

    fn mock_mget_transactions(starting_version: u64, count: usize, txn_size: usize) -> MockCmd {
        let keys: Vec<String> = (0..count).map(|i| {
            let version = starting_version + i as u64;
            let cache_key = CacheEntry::build_key(version, StorageFormat::GzipCompressedProto);
            cache_key
        }).collect();
        let values = (0..count).map(|i|
            redis::Value::Data(generate_transaction_bytes(starting_version + i as u64, txn_size))
        ).collect();
        let redis_resp = redis::Value::Bulk(values);
        MockCmd::new(
            redis::cmd("MGET").arg(keys),
            Ok(redis_resp),
        )
    }

    fn generate_redis_commands(transaction_size: u64) -> Vec<MockCmd> {
        let mut redis_commands = vec![];
        // No warm up.
        redis_commands.push(mock_get_latest_version(0));

        // Wait for a little bit since we're spawning up clients.
        redis_commands.push(mock_get_latest_version(0));
        // number of batches.
        let num_batches = 100 * 1024 * 1024 / (transaction_size as usize);
        // Start to update.
        for i in 0..num_batches {
            redis_commands.push(mock_get_latest_version((i as i64 + 1) * MAX_REDIS_FETCH_BATCH_SIZE as i64));
            redis_commands.push(mock_mget_transactions(i as u64 * (MAX_REDIS_FETCH_BATCH_SIZE as u64) ,
                MAX_REDIS_FETCH_BATCH_SIZE,
                transaction_size as usize));
        }

        redis_commands
    }



    // Creates a mock Redis connection with a stream of commands.
    // The pattern is:
    // 1. Warm up: get latest_version, get transactions.
    // 2. loop of fetching the data
    fn create_mock_redis(transaction_size: u64) -> MockRedisConnection {
        let mock_redis = MockRedisConnection::new(generate_redis_commands(transaction_size));
        mock_redis
    }

    // Spawn a task that consumes the cache. Return tps as a result.
    fn spawn_cache_consumption_task(cache: Arc<InMemoryCache>, duration_in_secs: u64) -> tokio::task::JoinHandle<f64>{
        tokio::spawn(async move {
            let start = std::time::Instant::now();
            let mut total_consumed_transactions = 0;
            let mut current_version = 0;
            while start.elapsed().as_secs() < duration_in_secs {
                let txns = cache.get_transactions(current_version).await;
                for txn in txns {
                    assert_eq!(txn.version, current_version);
                    total_consumed_transactions += 1;
                    current_version = txn.version + 1;
                }
                // Dropped here.
            }
            total_consumed_transactions as f64 / start.elapsed().as_secs_f64() as f64
        })
    }


    #[tokio::test]
    async fn test_small_transactions() {
        let transaction_size = 2 * 1024;
        let task_count = 10;
        let redis_connection = create_mock_redis(transaction_size);
        let cache = Arc::new(InMemoryCache::new_with_redis_connection(redis_connection, StorageFormat::GzipCompressedProto).await.unwrap());
        let tasks = (0..task_count).map(|_| spawn_cache_consumption_task(cache.clone(), 20)).collect::<Vec<_>>();
        // join all the tasks.
        let tps = futures::future::join_all(tasks).await.iter().map(|r| r.as_ref().unwrap()).sum::<f64>() / task_count as f64;
        println!("TPS: {}", tps);
        assert!(tps > 20_000.0);
    }

    #[tokio::test]
    async fn test_small_transactions_with_contention() {
        let transaction_size = 2 * 1024;
        let task_count = 1000;
        let redis_connection = create_mock_redis(transaction_size);
        let cache = Arc::new(InMemoryCache::new_with_redis_connection(redis_connection, StorageFormat::GzipCompressedProto).await.unwrap());
        let tasks = (0..task_count).map(|_| spawn_cache_consumption_task(cache.clone(), 20)).collect::<Vec<_>>();
        // join all the tasks.
        let tps = futures::future::join_all(tasks).await.iter().map(|r| r.as_ref().unwrap()).sum::<f64>() / task_count as f64;
        println!("TPS: {}", tps);
        assert!(tps > 100.0);
    }

    #[tokio::test]
    async fn test_large_transactions() {
        let transaction_size = 10 * 1024 * 1024;
        let task_count = 1000;
        let redis_connection = create_mock_redis(transaction_size);
        let cache = Arc::new(InMemoryCache::new_with_redis_connection(redis_connection, StorageFormat::GzipCompressedProto).await.unwrap());
        let tasks = (0..task_count).map(|_| spawn_cache_consumption_task(cache.clone(), 5)).collect::<Vec<_>>();
        // join all the tasks.
        let tps = futures::future::join_all(tasks).await.iter().map(|r| r.as_ref().unwrap()).sum::<f64>() / task_count as f64;
        println!("TPS: {}", tps);
        assert!(tps > 1011110.0);
    }

}
