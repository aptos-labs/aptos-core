// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{
    LATEST_PROCESSED_VERSION as LATEST_PROCESSED_VERSION_OLD, PROCESSED_VERSIONS_COUNT,
};
use anyhow::{bail, Context, Result};
use aptos_indexer_grpc_utils::{
    cache_operator::{CacheBatchGetStatus, CacheOperator},
    config::IndexerGrpcFileStoreConfig,
    constants::BLOB_STORAGE_SIZE,
    counters::{
        IndexerGrpcStep, DURATION_IN_SECS, LATEST_PROCESSED_VERSION, NUM_TRANSACTIONS_COUNT,
        TRANSACTION_UNIX_TIMESTAMP,
    },
    file_store_operator::{FileStoreOperator, GcsFileStoreOperator, LocalFileStoreOperator},
    storage::StorageFormat,
    time_diff_since_pb_timestamp_in_secs,
    types::RedisUrl,
};
use aptos_moving_average::MovingAverage;
use aptos_protos::transaction::v1::Transaction;
use std::time::Duration;
use tracing::info;

const SERVICE_TYPE: &str = "file_worker";

// If the version is ahead of the cache head, retry after a short sleep.
const AHEAD_OF_CACHE_SLEEP_DURATION_IN_MILLIS: u64 = 100;

/// Processor tails the data in cache and stores the data in file store.
pub struct Processor {
    cache_operator: CacheOperator<redis::aio::ConnectionManager>,
    file_store_operator: Box<dyn FileStoreOperator>,
    cache_chain_id: u64,

    #[allow(dead_code)]
    cache_storage_format: StorageFormat,
    #[allow(dead_code)]
    file_storage_format: StorageFormat,
}

impl Processor {
    pub async fn new(
        redis_main_instance_address: RedisUrl,
        file_store_config: IndexerGrpcFileStoreConfig,
        cache_storage_format: StorageFormat,
        file_storage_format: StorageFormat,
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

        let mut cache_operator = CacheOperator::new(conn, cache_storage_format);
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
                    file_storage_format,
                ))
            },
            IndexerGrpcFileStoreConfig::LocalFileStore(local_file_store) => {
                Box::new(LocalFileStoreOperator::new(
                    local_file_store.local_file_store_path.clone(),
                    file_storage_format,
                ))
            },
        };
        file_store_operator.verify_storage_bucket_existence().await;

        Ok(Self {
            cache_operator,
            file_store_operator,
            cache_chain_id,
            cache_storage_format,
            file_storage_format,
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
        let mut transactions_buffer: Vec<Transaction> = vec![];
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
                .batch_get_transactions(current_cache_version)
                .await;

            let current_transactions = match batch_get_result {
                Ok(CacheBatchGetStatus::Ok(transactions)) => transactions,
                Ok(CacheBatchGetStatus::NotReady) => {
                    // If the cache is not ready, we sleep for a short time and retry.
                    tokio::time::sleep(Duration::from_millis(
                        AHEAD_OF_CACHE_SLEEP_DURATION_IN_MILLIS,
                    ))
                    .await;
                    continue;
                },
                Ok(CacheBatchGetStatus::EvictedFromCache) => {
                    bail!("Evicted from cache.")
                },
                Err(err) => bail!("Error when fetching transactions from cache: {}", err),
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
            let current_batch: Vec<Transaction> =
                transactions_buffer.drain(..process_size).collect();
            // Hack: Though the copy is expensive here but we want to inspect the transaction timestamp in the log without
            // lifetime issue.
            let first_transaction_pb_timestamp = current_batch
                .as_slice()
                .first()
                .unwrap()
                .timestamp
                .as_ref()
                .unwrap()
                .clone();
            let last_transaction_pb_timestamp = current_batch
                .as_slice()
                .last()
                .unwrap()
                .timestamp
                .as_ref()
                .unwrap()
                .clone();
            self.file_store_operator
                .upload_transactions(cache_chain_id, current_batch)
                .await
                .context("Uploading transactions to file store failed.")?;
            PROCESSED_VERSIONS_COUNT.inc_by(process_size as u64);
            tps_calculator.tick_now(process_size as u64);
            let end_version = current_file_store_version + process_size as u64 - 1_u64;
            let num_transactions = end_version - current_file_store_version + 1;
            let start_version_txn_latency =
                time_diff_since_pb_timestamp_in_secs(&first_transaction_pb_timestamp);
            let end_version_txn_latency =
                time_diff_since_pb_timestamp_in_secs(&last_transaction_pb_timestamp);
            info!(
                start_version = current_file_store_version,
                end_version = end_version,
                start_txn_timestamp_iso = start_version_timestamp
                    .clone()
                    .map(|t| timestamp_to_iso(&t))
                    .unwrap_or_default(),
                end_txn_timestamp_iso = end_version_timestamp
                    .map(|t| timestamp_to_iso(&t))
                    .unwrap_or_default(),
                num_of_transactions = num_transactions,
                duration_in_secs = file_store_upload_batch_start.elapsed().as_secs_f64(),
                tps = (tps_calculator.avg() * 1000.0) as u64,
                current_file_store_version = current_file_store_version,
                service_type = SERVICE_TYPE,
                step = IndexerGrpcStep::FilestoreUploadTxns.get_step(),
                "{}",
                IndexerGrpcStep::FilestoreUploadTxns.get_label(),
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
                    start_version_timestamp
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
