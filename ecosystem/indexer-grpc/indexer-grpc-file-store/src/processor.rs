// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{
    LATEST_PROCESSED_VERSION as LATEST_PROCESSED_VERSION_OLD, PROCESSED_VERSIONS_COUNT,
};
use anyhow::{bail, Context, Result};
use aptos_indexer_grpc_utils::{
    build_protobuf_encoded_transaction_wrappers,
    cache_operator::{CacheBatchGetStatus, CacheOperator},
    config::IndexerGrpcFileStoreConfig,
    constants::BLOB_STORAGE_SIZE,
    counters::{
        IndexerGrpcStep, DURATION_IN_SECS, LATEST_PROCESSED_VERSION, NUM_TRANSACTIONS_COUNT,
        TRANSACTION_UNIX_TIMESTAMP,
    },
    file_store_operator::{FileStoreOperator, GcsFileStoreOperator, LocalFileStoreOperator},
    timestamp_to_unixtime,
    types::RedisUrl,
    EncodedTransactionWithVersion,
};
use aptos_moving_average::MovingAverage;
use aptos_protos::transaction::v1::Transaction;
use prost::Message;
use std::time::Duration;
use tracing::info;

// If the version is ahead of the cache head, retry after a short sleep.
const AHEAD_OF_CACHE_SLEEP_DURATION_IN_MILLIS: u64 = 100;
const SERVICE_TYPE: &str = "file_worker";

/// Processor tails the data in cache and stores the data in file store.
pub struct Processor {
    cache_operator: CacheOperator<redis::aio::ConnectionManager>,
    file_store_operator: Box<dyn FileStoreOperator>,
    cache_chain_id: u64,
}

impl Processor {
    pub async fn new(
        redis_main_instance_address: RedisUrl,
        file_store_config: IndexerGrpcFileStoreConfig,
    ) -> Result<Self> {
        // Connection to redis is a hard dependency for file store processor.
        let conn = redis::Client::open(redis_main_instance_address.0.clone())
            .with_context(|| {
                format!(
                    "Create redis client for {} failed",
                    redis_main_instance_address.0
                )
            })?
            .get_tokio_connection_manager()
            .await
            .with_context(|| {
                format!(
                    "Create redis connection to {} failed.",
                    redis_main_instance_address.0
                )
            })?;

        let mut cache_operator = CacheOperator::new(conn);
        let cache_chain_id = cache_operator
            .get_chain_id()
            .await
            .context("Get chain id failed.")?;

        let file_store_operator: Box<dyn FileStoreOperator> = match &file_store_config {
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

        Ok(Self {
            cache_operator,
            file_store_operator,
            cache_chain_id,
        })
    }

    // Starts the processing.
    pub async fn run(&mut self) -> Result<()> {
        let cache_chain_id = self.cache_chain_id;

        // If file store and cache chain id don't match, return an error.
        let metadata = self
            .file_store_operator
            .create_default_file_store_metadata_if_absent(cache_chain_id)
            .await
            .context("Metadata did not match.")?;

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
                bail!("File store version is not a multiple of BLOB_STORAGE_SIZE.");
            }

            let file_store_upload_batch_start = std::time::Instant::now();
            let batch_get_result = self
                .cache_operator
                .batch_get_encoded_proto_data(current_cache_version)
                .await;

            let batch_get_result =
                fullnode_grpc_status_handling(batch_get_result, current_cache_version)?;

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
            let current_batch: Vec<EncodedTransactionWithVersion> =
                transactions_buffer.drain(..process_size).collect();
<<<<<<< HEAD
            let last_transaction = current_batch.as_slice().last().unwrap().clone();
=======
            let first_transaction = current_batch.as_slice().first().unwrap().clone();
>>>>>>> 0c0f43d9a0 ([indexer grpc] Add metrics)
            self.file_store_operator
                .upload_transactions(cache_chain_id, current_batch)
                .await
                .context("Uploading transactions to file store failed.")?;
            PROCESSED_VERSIONS_COUNT.inc_by(process_size as u64);
            tps_calculator.tick_now(process_size as u64);
            let end_version = current_file_store_version + process_size as u64 - 1_u64;
            let num_transactions = end_version - current_file_store_version + 1;
            // This decoding may be inefficient, but this is the file store so we don't have to be overly
            // concerned with efficiency.
<<<<<<< HEAD
            let end_version_timestamp = {
                let encoded_transaction = last_transaction.0;
=======
            let start_version_timestamp = {
                let encoded_transaction = first_transaction.0;
>>>>>>> 0c0f43d9a0 ([indexer grpc] Add metrics)
                let decoded_transaction =
                    base64::decode(encoded_transaction).expect("Failed to decode base64.");
                let transaction =
                    Transaction::decode(&*decoded_transaction).expect("Failed to decode protobuf.");
                transaction.timestamp
            };

            info!(
                tps = (tps_calculator.avg() * 1000.0) as u64,
                current_file_store_version = current_file_store_version,
                "Upload transactions to file store."
            );

            LATEST_PROCESSED_VERSION
                .with_label_values(&[
                    SERVICE_TYPE,
                    IndexerGrpcStep::FilestoreUploadTxns.get_step(),
                    IndexerGrpcStep::FilestoreUploadTxns.get_label(),
                ])
                .set(end_version as i64);
            TRANSACTION_UNIX_TIMESTAMP
                .with_label_values(&[
                    SERVICE_TYPE,
                    IndexerGrpcStep::FilestoreUploadTxns.get_step(),
                    IndexerGrpcStep::FilestoreUploadTxns.get_label(),
                ])
                .set(
<<<<<<< HEAD
                    end_version_timestamp
=======
                    start_version_timestamp
>>>>>>> 0c0f43d9a0 ([indexer grpc] Add metrics)
                        .map(|t| timestamp_to_unixtime(&t))
                        .unwrap_or_default(),
                );
            NUM_TRANSACTIONS_COUNT
                .with_label_values(&[
                    SERVICE_TYPE,
                    IndexerGrpcStep::FilestoreUploadTxns.get_step(),
                    IndexerGrpcStep::FilestoreUploadTxns.get_label(),
                ])
                .set(num_transactions as i64);
            DURATION_IN_SECS
                .with_label_values(&[
                    SERVICE_TYPE,
                    IndexerGrpcStep::FilestoreUploadTxns.get_step(),
                    IndexerGrpcStep::FilestoreUploadTxns.get_label(),
                ])
                .set(file_store_upload_batch_start.elapsed().as_secs_f64());

            current_file_store_version += process_size as u64;
            LATEST_PROCESSED_VERSION_OLD.set(current_file_store_version as i64);
        }
    }
}

fn fullnode_grpc_status_handling(
    fullnode_rpc_status: anyhow::Result<CacheBatchGetStatus>,
    batch_start_version: u64,
) -> Result<Option<Vec<EncodedTransactionWithVersion>>> {
    match fullnode_rpc_status {
        Ok(CacheBatchGetStatus::Ok(encoded_transactions)) => Ok(Some(
            build_protobuf_encoded_transaction_wrappers(encoded_transactions, batch_start_version),
        )),
        Ok(CacheBatchGetStatus::NotReady) => Ok(None),
        Ok(CacheBatchGetStatus::EvictedFromCache) => {
            bail!(
                "[indexer file] Cache evicted from cache. For file store worker, this is not expected."
            );
        },
        Err(err) => {
            bail!("Batch get encoded proto data failed: {}", err);
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
        assert!(
            fullnode_grpc_status_handling(fullnode_rpc_status, batch_start_version)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn verify_the_grpc_status_handling_evicted_from_cache() {
        let fullnode_rpc_status: anyhow::Result<CacheBatchGetStatus> =
            Ok(CacheBatchGetStatus::EvictedFromCache);
        let batch_start_version = 0;
        assert!(fullnode_grpc_status_handling(fullnode_rpc_status, batch_start_version).is_err());
    }

    #[test]
    fn verify_the_grpc_status_handling_error() {
        let fullnode_rpc_status: anyhow::Result<CacheBatchGetStatus> =
            Err(anyhow::anyhow!("Error"));
        let batch_start_version = 0;
        assert!(fullnode_grpc_status_handling(fullnode_rpc_status, batch_start_version).is_err());
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
            fullnode_grpc_status_handling(fullnode_rpc_status, batch_start_version).unwrap();
        assert!(actual_transactions.is_some());
        let actual_transactions = actual_transactions.unwrap();
        assert_eq!(actual_transactions, transactions_with_version);
    }
}
