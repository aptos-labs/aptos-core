// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_utils::{
    cache_operator::{CacheBatchGetStatus, CacheOperator},
    config::IndexerGrpcConfig,
    constants::BLOB_STORAGE_SIZE,
    file_store_operator::FileStoreOperator,
};
use aptos_moving_average::MovingAverage;
use std::{thread::sleep, time::Duration};

/// Processor tails the data in cache and stores the data in file store.
pub struct Processor {
    cache_operator: Option<CacheOperator<redis::aio::Connection>>,
    file_store_processor: Option<FileStoreOperator>,
    cache_chain_id: Option<u64>,
    config: IndexerGrpcConfig,
}

impl Processor {
    pub fn new(config: IndexerGrpcConfig) -> Self {
        Self {
            cache_operator: None,
            file_store_processor: None,
            cache_chain_id: None,
            config,
        }
    }

    /// Bootstrap the processor, including creating the redis connection and file store operator.
    async fn bootstrap(&mut self) {
        // Connection to redis is a hard dependency for file store processor.
        let conn = redis::Client::open(format!("redis://{}", self.config.redis_address))
            .expect("Create redis client failed.")
            .get_async_connection()
            .await
            .expect("Create redis connection failed.");

        let mut cache_operator = CacheOperator::new(conn);
        let chain_id = cache_operator
            .get_chain_id()
            .await
            .expect("Get chain id failed.");

        let file_store_operator =
            FileStoreOperator::new(self.config.file_store_bucket_name.clone());
        file_store_operator.bootstrap().await;

        self.cache_operator = Some(cache_operator);
        self.file_store_processor = Some(file_store_operator);
        self.cache_chain_id = Some(chain_id);
    }

    // Starts the processing.
    pub async fn run(&mut self) {
        self.bootstrap().await;
        let cache_chain_id = self.cache_chain_id.unwrap();

        // If file store and cache chain id don't match, panic.
        let metadata = self
            .file_store_processor
            .as_mut()
            .unwrap()
            .create_default_file_store_metadata_if_absent(cache_chain_id)
            .await
            .unwrap();

        // The version to fetch from cache.
        let mut current_cache_version = metadata.version;
        let mut current_file_store_version = current_cache_version;
        // The transactions buffer.
        let mut transactions: Vec<String> = vec![];
        let mut ma = MovingAverage::new(10_000);
        // Once we hit the head, the processing is slowed to single thread.
        let mut hit_head = false;

        loop {
            let batch_get_result = self
                .cache_operator
                .as_mut()
                .unwrap()
                .batch_get_encoded_proto_data(current_cache_version)
                .await;

            match batch_get_result {
                Ok(CacheBatchGetStatus::Ok(t)) => {
                    current_cache_version += t.len() as u64;
                    transactions.extend(t);
                },
                Ok(CacheBatchGetStatus::NotReady) => {
                    sleep(Duration::from_secs(1));
                    aptos_logger::info!(
                        current_file_store_version = current_file_store_version,
                        current_cache_version = current_cache_version,
                        "Cache is not ready. Sleep for 1 second."
                    );
                    continue;
                },
                Ok(CacheBatchGetStatus::HitTheHead(t)) => {
                    current_cache_version += t.len() as u64;
                    transactions.extend(t);
                    hit_head = true;
                    aptos_logger::info!(
                        current_file_store_version = current_file_store_version,
                        current_cache_version = current_cache_version,
                        "File store processor hits the head."
                    );
                },
                Ok(CacheBatchGetStatus::EvictedFromCache) => {
                    panic!(
                        "Cache evicted from cache. For file store worker, this is not expected."
                    );
                },
                Err(err) => {
                    panic!("Batch get encoded proto data failed: {}", err);
                },
            }
            // If not hit the head, we want to collect more transactions.
            if !hit_head && transactions.len() < 10 * BLOB_STORAGE_SIZE {
                // If we haven't hit the head, we want to collect more transactions.
                continue;
            }
            // If hit the head, we want to collect at least one batch of transactions.
            if hit_head && transactions.len() < BLOB_STORAGE_SIZE {
                continue;
            }
            let batch_size = match !hit_head && transactions.len() >= 10 * BLOB_STORAGE_SIZE {
                true => 10 * BLOB_STORAGE_SIZE,
                false => BLOB_STORAGE_SIZE,
            };
            let current_batch: Vec<String> = transactions.drain(..batch_size).collect();
            self.file_store_processor
                .as_mut()
                .unwrap()
                .upload_transactions(cache_chain_id, current_file_store_version, current_batch)
                .await
                .unwrap();
            ma.tick_now(batch_size as u64);
            aptos_logger::info!(
                tps = (ma.avg() * 1000.0) as u64,
                current_file_store_version = current_file_store_version,
                "Upload transactions to file store."
            );
            current_file_store_version += batch_size as u64;
        }
    }
}
