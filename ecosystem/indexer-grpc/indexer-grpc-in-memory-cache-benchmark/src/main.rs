// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This benchmark is used to test the performance of in-memory cache.
// Key performance requirements:
//   read speed
//      * 20k reads per second for small load -  2 KB per txn for 10 clients.  - TPS, ~4 Gbps.
//      * 100 reads per second for large load -  1 MB per txn for 10 clients   - memory-(de)allocation, ~10 Gbps.
//      * 100 reads per second for small load -  2 KB per txn for 1000 clients - contended reads, ~2 Gbps.

use velor_indexer_grpc_utils::{
    compression_util::{CacheEntry, StorageFormat},
    in_memory_cache::{InMemoryCache, InMemoryCacheConfig, MAX_REDIS_FETCH_BATCH_SIZE},
};
use velor_protos::transaction::v1::{Transaction, TransactionInfo};
use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
use redis_test::{MockCmd, MockRedisConnection};
use std::sync::Arc;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

lazy_static! {
    static ref TRANSACTION_CONTENT: OnceCell<Vec<u8>> = OnceCell::new();
}
// Create a transaction with a given version and size.
// This involves memory allocation.
fn generate_transaction_bytes(version: u64, size: usize) -> Vec<u8> {
    let txn = Transaction {
        version,
        info: Some(TransactionInfo {
            hash: TRANSACTION_CONTENT.get_or_init(|| vec![1; size]).clone(),
            ..TransactionInfo::default()
        }),
        ..Transaction::default()
    };
    let storage_format = StorageFormat::Lz4CompressedProto;
    let cache_entry = CacheEntry::from_transaction(txn, storage_format);
    cache_entry.into_inner()
}

fn mock_get_latest_version(version: i64) -> MockCmd {
    MockCmd::new(redis::cmd("GET").arg("latest_version"), Ok(version))
}

fn mock_mget_transactions(starting_version: u64, count: usize, txn_size: usize) -> MockCmd {
    let keys: Vec<String> = (0..count)
        .map(|i| {
            let version = starting_version + i as u64;
            CacheEntry::build_key(version, StorageFormat::Lz4CompressedProto)
        })
        .collect();
    let values = (0..count)
        .map(|i| {
            redis::Value::Data(generate_transaction_bytes(
                starting_version + i as u64,
                txn_size,
            ))
        })
        .collect();
    let redis_resp = redis::Value::Bulk(values);
    MockCmd::new(redis::cmd("MGET").arg(keys), Ok(redis_resp))
}

fn generate_redis_commands(transaction_size: u64) -> Vec<MockCmd> {
    let mut redis_commands = vec![
        // No warm up.
        mock_get_latest_version(0),
        // Wait for a little bit since we're spawning up clients.
        mock_get_latest_version(0),
        mock_get_latest_version(0),
        mock_get_latest_version(0),
        mock_get_latest_version(0),
        mock_get_latest_version(0),
    ];

    // number of batches.
    let num_batches = 100 * 1024 * 1024 / (transaction_size as usize);
    // Start to update.
    for i in 0..num_batches {
        redis_commands.push(mock_get_latest_version(
            (i as i64 + 1) * MAX_REDIS_FETCH_BATCH_SIZE as i64,
        ));
        redis_commands.push(mock_mget_transactions(
            i as u64 * (MAX_REDIS_FETCH_BATCH_SIZE as u64),
            MAX_REDIS_FETCH_BATCH_SIZE,
            transaction_size as usize,
        ));
    }

    redis_commands
}

// Creates a mock Redis connection with a stream of commands.
// The pattern is:
// 1. Warm up: get latest_version, get transactions.
// 2. loop of fetching the data
fn create_mock_redis(transaction_size: u64) -> MockRedisConnection {
    MockRedisConnection::new(generate_redis_commands(transaction_size))
}

// Spawn a task that consumes the cache. Return tps as a result.
fn spawn_cache_consumption_task(
    cache: Arc<InMemoryCache>,
    duration_in_secs: u64,
) -> tokio::task::JoinHandle<f64> {
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
        total_consumed_transactions as f64 / start.elapsed().as_secs_f64()
    })
}

async fn run_transaction_test(
    transaction_size: u64,
    task_count: u64,
    duration_in_secs: u64,
    expected_tps: f64,
) {
    let redis_connection = create_mock_redis(transaction_size);
    let cache = Arc::new(
        InMemoryCache::new_with_redis_connection(
            InMemoryCacheConfig::default(),
            redis_connection,
            StorageFormat::Lz4CompressedProto,
        )
        .await
        .unwrap(),
    );
    let tasks = (0..task_count)
        .map(|_| spawn_cache_consumption_task(cache.clone(), duration_in_secs))
        .collect::<Vec<_>>();
    // join all the tasks.
    let tps = futures::future::join_all(tasks)
        .await
        .iter()
        .map(|r| r.as_ref().unwrap())
        .sum::<f64>()
        / task_count as f64;
    println!("TPS: {}", tps);
    assert!(tps > expected_tps);
}

#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
async fn main() -> anyhow::Result<()> {
    run_transaction_test(2 * 1024, 10, 20, 20_000.0).await;
    run_transaction_test(2 * 1024, 1000, 20, 100.0).await;
    run_transaction_test(1024 * 1024, 20, 5, 500.0).await;
    Ok(())
}
