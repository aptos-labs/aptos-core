// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{LATEST_PROCESSED_VERSION, PROCESSED_VERSIONS_COUNT};
use aptos_indexer_grpc_utils::{
    build_protobuf_encoded_transaction_wrappers,
    cache_operator::{CacheBatchGetStatus, CacheOperator},
    config::IndexerGrpcFileStoreConfig,
    constants::BLOB_STORAGE_SIZE,
    file_store_operator::{FileStoreOperator, GcsFileStoreOperator, LocalFileStoreOperator},
    EncodedTransactionWithVersion,
};
use aptos_moving_average::MovingAverage;
use std::time::Duration;
use tracing::info;

// If the version is ahead of the cache head, retry after a short sleep.
const AHEAD_OF_CACHE_SLEEP_DURATION_IN_MILLIS: u64 = 100;

/// Processor tails the data in cache and stores the data in file store.
pub struct Processor {
    cache_operator: Option<CacheOperator<redis::aio::ConnectionManager>>,
    file_store_processor: Option<Box<dyn FileStoreOperator>>,
    cache_chain_id: Option<u64>,
    redis_main_instance_address: String,
    file_store_config: IndexerGrpcFileStoreConfig,
}

impl Processor {
    pub fn new(
        redis_main_instance_address: String,
        file_store_config: IndexerGrpcFileStoreConfig,
    ) -> Self {
        Self {
            cache_operator: None,
            file_store_processor: None,
            cache_chain_id: None,
            redis_main_instance_address,
            file_store_config,
        }
    }

    /// Init the processor, including creating the redis connection and file store operator.
    async fn init(&mut self) {
        // Connection to redis is a hard dependency for file store processor.
        let conn = redis::Client::open(format!("redis://{}", self.redis_main_instance_address))
            .expect("Create redis client failed.")
            .get_tokio_connection_manager()
            .await
            .expect("Create redis connection failed.");

        let mut cache_operator = CacheOperator::new(conn);
        let chain_id = cache_operator
            .get_chain_id()
            .await
            .expect("Get chain id failed.");

        let file_store_operator: Box<dyn FileStoreOperator> = match &self.file_store_config {
            IndexerGrpcFileStoreConfig::GcsFileStore(gcs_file_store) => {
                Box::new(GcsFileStoreOperator::new(
                    gcs_file_store.gcs_file_store_bucket_name.clone(),
                    gcs_file_store
                        .gcs_file_store_service_account_key_path
                        .clone(),
                ))
            },
            IndexerGrpcFileStoreConfig::LocalFileStore(local_file_store) => Box::new(
                LocalFileStoreOperator::new(local_file_store.local_file_store_path.clone()),
            ),
        };
        file_store_operator.verify_storage_bucket_existence().await;

        self.cache_operator = Some(cache_operator);
        self.file_store_processor = Some(file_store_operator);
        self.cache_chain_id = Some(chain_id);
    }

    // Starts the processing.
    pub async fn run(&mut self) {
        self.init().await;
        let cache_chain_id = self.cache_chain_id.unwrap();

        // If file store and cache chain id don't match, panic.
        let metadata = self
            .file_store_processor
            .as_mut()
            .unwrap()
            .create_default_file_store_metadata_if_absent(cache_chain_id)
            .await
            .unwrap();

        // This implements a two-cursor approach:
        //   * One curosr is to track the current cache version.
        //   * The other cursor is to track the current file store version.
        //   * Constrains:
        //     * The current cache version >= the current file store version.
        //     * The current file store version is always a multiple of BLOB_STORAGE_SIZE.
        let mut current_cache_version = metadata.version;
        let mut current_file_store_version = current_cache_version;
        // The transactions buffer to store the transactions fetched from cache.
        let mut transactions_buffer: Vec<EncodedTransactionWithVersion> = vec![];
        let mut tps_calculator = MovingAverage::new(10_000);
        loop {
            // 0. Data verfiication.
            // File store version has to be a multiple of BLOB_STORAGE_SIZE.
            if current_file_store_version % BLOB_STORAGE_SIZE as u64 != 0 {
                panic!("File store version is not a multiple of BLOB_STORAGE_SIZE.");
            }

            let batch_get_result = self
                .cache_operator
                .as_mut()
                .unwrap()
                .batch_get_encoded_proto_data(current_cache_version)
                .await;

            let batch_get_result =
                fullnode_grpc_status_handling(batch_get_result, current_cache_version);

            let current_transactions = match batch_get_result {
                Some(transactions) => transactions,
                None => {
                    // Cache is not ready yet, i.e., ahead of current head. Wait.
                    tokio::time::sleep(Duration::from_millis(
                        AHEAD_OF_CACHE_SLEEP_DURATION_IN_MILLIS,
                    ))
                    .await;
                    continue;
                },
            };

            let hit_head = current_transactions.len() != BLOB_STORAGE_SIZE;
            // Update the current cache version.
            current_cache_version += current_transactions.len() as u64;
            transactions_buffer.extend(current_transactions);

            // If not hit the head, we want to collect more transactions.
            if !hit_head && transactions_buffer.len() < 10 * BLOB_STORAGE_SIZE {
                // If we haven't hit the head, we want to collect more transactions.
                continue;
            }
            // If hit the head, we want to collect at least one batch of transactions.
            if hit_head && transactions_buffer.len() < BLOB_STORAGE_SIZE {
                continue;
            }
            // Drain the transactions buffer and upload to file store in size of multiple of BLOB_STORAGE_SIZE.
            let process_size = transactions_buffer.len() / BLOB_STORAGE_SIZE * BLOB_STORAGE_SIZE;
            let current_batch = transactions_buffer.drain(..process_size).collect();

            self.file_store_processor
                .as_mut()
                .unwrap()
                .upload_transactions(cache_chain_id, current_batch)
                .await
                .unwrap();
            PROCESSED_VERSIONS_COUNT.inc_by(process_size as u64);
            tps_calculator.tick_now(process_size as u64);
            info!(
                tps = (tps_calculator.avg() * 1000.0) as u64,
                current_file_store_version = current_file_store_version,
                "Upload transactions to file store."
            );
            current_file_store_version += process_size as u64;
            LATEST_PROCESSED_VERSION.set(current_file_store_version as i64);
        }
    }
}

fn fullnode_grpc_status_handling(
    fullnode_rpc_status: anyhow::Result<CacheBatchGetStatus>,
    batch_start_version: u64,
) -> Option<Vec<EncodedTransactionWithVersion>> {
    match fullnode_rpc_status {
        Ok(CacheBatchGetStatus::Ok(encoded_transactions)) => Some(
            build_protobuf_encoded_transaction_wrappers(encoded_transactions, batch_start_version),
        ),
        Ok(CacheBatchGetStatus::NotReady) => None,
        Ok(CacheBatchGetStatus::EvictedFromCache) => {
            panic!(
                "[indexer file]Cache evicted from cache. For file store worker, this is not expected."
            );
        },
        Err(err) => {
            panic!("Batch get encoded proto data failed: {}", err);
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_the_grpc_status_handling_ahead_of_cache() {
        let fullnode_rpc_status: anyhow::Result<CacheBatchGetStatus> =
            Ok(CacheBatchGetStatus::NotReady);
        let batch_start_version = 0;
        assert!(fullnode_grpc_status_handling(fullnode_rpc_status, batch_start_version).is_none());
    }

    #[test]
    #[should_panic]
    fn verify_the_grpc_status_handling_evicted_from_cache() {
        let fullnode_rpc_status: anyhow::Result<CacheBatchGetStatus> =
            Ok(CacheBatchGetStatus::EvictedFromCache);
        let batch_start_version = 0;
        fullnode_grpc_status_handling(fullnode_rpc_status, batch_start_version);
    }

    #[test]
    #[should_panic]
    fn verify_the_grpc_status_handling_error() {
        let fullnode_rpc_status: anyhow::Result<CacheBatchGetStatus> =
            Err(anyhow::anyhow!("Error"));
        let batch_start_version = 0;
        fullnode_grpc_status_handling(fullnode_rpc_status, batch_start_version);
    }

    #[test]
    fn verify_the_grpc_status_handling_ok() {
        let batch_start_version = 2000;
        let transactions: Vec<String> = std::iter::repeat("txn".to_string()).take(1000).collect();
        let transactions_with_version: Vec<EncodedTransactionWithVersion> = transactions
            .iter()
            .enumerate()
            .map(|(index, txn)| (txn.clone(), batch_start_version + index as u64))
            .collect();
        let fullnode_rpc_status: anyhow::Result<CacheBatchGetStatus> =
            Ok(CacheBatchGetStatus::Ok(transactions));
        let actual_transactions =
            fullnode_grpc_status_handling(fullnode_rpc_status, batch_start_version);
        assert!(actual_transactions.is_some());
        let actual_transactions = actual_transactions.unwrap();
        assert_eq!(actual_transactions, transactions_with_version);
    }
}
