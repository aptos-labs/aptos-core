// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::PROCESSED_VERSIONS_COUNT;
use anyhow::{bail, Context, Result};
use aptos_indexer_grpc_utils::{
    build_protobuf_encoded_transaction_wrappers,
    cache_operator::{CacheBatchGetStatus, CacheOperator},
    config::IndexerGrpcFileStoreConfig,
    constants::BLOB_STORAGE_SIZE,
    counters::{log_grpc_step, IndexerGrpcStep},
    file_store_operator::{FileStoreOperator, GcsFileStoreOperator, LocalFileStoreOperator},
    types::RedisUrl,
    EncodedTransactionWithVersion,
};
use aptos_moving_average::MovingAverage;
use aptos_protos::transaction::v1::Transaction;
use prost::Message;
use std::time::Duration;
use tracing::{debug, info};

// If the version is ahead of the cache head, retry after a short sleep.
const AHEAD_OF_CACHE_SLEEP_DURATION_IN_MILLIS: u64 = 100;
const LARGE_FILE_BYTES_COUNT: usize = 100_000_000;
const SERVICE_TYPE: &str = "file_worker";

/// Processor tails the data in cache and stores the data in file store.
pub struct Processor {
    cache_operator: CacheOperator<redis::aio::ConnectionManager>,
    file_store_operator: Box<dyn FileStoreOperator>,
    cache_chain_id: u64,
    enable_verbose_logging: bool,
}

impl Processor {
    pub async fn new(
        redis_main_instance_address: RedisUrl,
        file_store_config: IndexerGrpcFileStoreConfig,
        enable_verbose_logging: bool,
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
            enable_verbose_logging,
        })
    }

    /// Starts the processing. The steps are
    /// 1. Check chain id at the beginning and every step after
    /// 2. Get the batch start version from file store metadata
    /// 3. Start loop
    ///   3.1 Check head from cache, decide whether we need to parallel process or just wait
    ///   3.2 If we're ready to process, create max of 10 threads and fetch / upload data
    ///   3.3 Update file store metadata at the end of a batch
    pub async fn run(&mut self) -> Result<()> {
        let cache_chain_id = self.cache_chain_id;

        let mut batch_start_version =
            if let Some(metadata) = self.file_store_operator.get_file_store_metadata().await {
                anyhow::ensure!(metadata.chain_id == cache_chain_id, "Chain ID mismatch.");
                metadata.version
            } else {
                0
            };

        let mut tps_calculator = MovingAverage::new(10_000);
        loop {
            let latest_time = std::time::Instant::now();
            let cache_worker_latest = self.cache_operator.get_latest_version().await?;

            // batches tracks the start version of the batches to fetch. 1000 at the time
            let mut batches = vec![];
            let mut start_version = batch_start_version;
            while start_version + (BLOB_STORAGE_SIZE as u64) < cache_worker_latest {
                batches.push(start_version);
                start_version += BLOB_STORAGE_SIZE as u64;
            }

            // we're too close to the head
            if batches.is_empty() {
                debug!(
                    batch_start_version = batch_start_version,
                    cache_worker_latest = cache_worker_latest,
                    "[Filestore] No enough version yet, need 1000 versions at least"
                );
                tokio::time::sleep(Duration::from_millis(
                    AHEAD_OF_CACHE_SLEEP_DURATION_IN_MILLIS,
                ))
                .await;
                continue;
            }

            // Create thread and fetch transactions
            let tasks = vec![];
            for start_version in batches {
                let cache_operator_clone = self.cache_operator.clone();
                let file_store_operator_clone = self.file_store_operator.clone();
                let task = tokio::spawn(async move {
                    let transactions = cache_operator_clone
                        .batch_get_encoded_proto_data_x(start_version, BLOB_STORAGE_SIZE as u64)
                        .await
                        .unwrap();
                    let (start, end) = file_store_operator_clone
                        .upload_transaction_batch(cache_chain_id, transactions)
                        .await
                        .unwrap();
                    (start, end, transactions.last().unwrap().0)
                });
                tasks.push(task);
            }
            let (first_version, last_version, last_version_encoded) =
                match futures::future::try_join_all(tasks).await {
                    Ok(res) => {
                        // Check for gaps
                        res.sort_by(|a, b| a.0.cmp(&b.0));
                        let mut prev_start = None;
                        let mut prev_end = None;
                        for result in res {
                            let start = result.0;
                            let end = result.1;
                            if prev_start.is_none() {
                                prev_start = Some(start);
                                prev_end = Some(end);
                            } else {
                                if prev_end.unwrap() + 1 != start {
                                    tracing::error!(
                                        processed_versions = ?res,
                                        "[Filestore] Gaps in processing data"
                                    );
                                    panic!("[Filestore] Gaps in processing data");
                                }
                                prev_start = Some(start);
                                prev_end = Some(end);
                            }
                        }
                        (
                            res.first().unwrap().0,
                            res.last().unwrap().1,
                            res.last().unwrap().2.clone(),
                        )
                    },
                    Err(err) => panic!("Error processing transaction batches: {:?}", err),
                };

            // update next batch start version
            batch_start_version = last_version + 1;
            assert!(
                batch_start_version % BLOB_STORAGE_SIZE as u64 == 0,
                "[Filestore] Batch must be multiple of 1000"
            );

            // write to filestore
            while self
                .file_store_operator
                .update_file_store_metadata_with_timeout(cache_chain_id, batch_start_version)
                .await
                .is_err()
            {
                tracing::error!(
                    batch_start_version = batch_start_version,
                    "Failed to update file store metadata. Retrying in 500ms."
                );
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            let size = last_version - first_version + 1;
            PROCESSED_VERSIONS_COUNT.inc_by(size);
            tps_calculator.tick_now(size);
            // This decoding may be inefficient, but this is the file store so we don't have to be overly
            // concerned with efficiency.
            let end_version_timestamp = {
                let decoded_transaction =
                    base64::decode(last_version_encoded).expect("Failed to decode base64.");
                let transaction =
                    Transaction::decode(&*decoded_transaction).expect("Failed to decode protobuf.");
                transaction.timestamp
            };

            let duration = latest_time.elapsed().as_secs_f64();
            info!(
                tps = (tps_calculator.avg() * 1000.0) as u64,
                start_version = first_version,
                end_version = last_version,
                duration_in_secs = duration,
                service_type = SERVICE_TYPE,
                "{}",
                IndexerGrpcStep::FilestoreUploadTxns.get_label()
            );

            LATEST_PROCESSED_VERSION
                .with_label_values(&[
                    SERVICE_TYPE,
                    IndexerGrpcStep::FilestoreUploadTxns.get_step(),
                    IndexerGrpcStep::FilestoreUploadTxns.get_label(),
                ])
                .set(last_version as i64);
            TRANSACTION_UNIX_TIMESTAMP
                .with_label_values(&[
                    SERVICE_TYPE,
                    IndexerGrpcStep::FilestoreUploadTxns.get_step(),
                    IndexerGrpcStep::FilestoreUploadTxns.get_label(),
                ])
                .set(
                    end_version_timestamp
                        .map(|t| timestamp_to_unixtime(&t))
                        .unwrap_or_default(),
                );
            NUM_TRANSACTIONS_COUNT
                .with_label_values(&[
                    SERVICE_TYPE,
                    IndexerGrpcStep::FilestoreUploadTxns.get_step(),
                    IndexerGrpcStep::FilestoreUploadTxns.get_label(),
                ])
                .set(size as i64);
            DURATION_IN_SECS
                .with_label_values(&[
                    SERVICE_TYPE,
                    IndexerGrpcStep::FilestoreUploadTxns.get_step(),
                    IndexerGrpcStep::FilestoreUploadTxns.get_label(),
                ])
                .set(duration);
        }
    }
}

fn handle_batch_from_cache(
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
            handle_batch_from_cache(fullnode_rpc_status, batch_start_version)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn verify_the_grpc_status_handling_evicted_from_cache() {
        let fullnode_rpc_status: anyhow::Result<CacheBatchGetStatus> =
            Ok(CacheBatchGetStatus::EvictedFromCache);
        let batch_start_version = 0;
        assert!(handle_batch_from_cache(fullnode_rpc_status, batch_start_version).is_err());
    }

    #[test]
    fn verify_the_grpc_status_handling_error() {
        let fullnode_rpc_status: anyhow::Result<CacheBatchGetStatus> =
            Err(anyhow::anyhow!("Error"));
        let batch_start_version = 0;
        assert!(handle_batch_from_cache(fullnode_rpc_status, batch_start_version).is_err());
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
            handle_batch_from_cache(fullnode_rpc_status, batch_start_version).unwrap();
        assert!(actual_transactions.is_some());
        let actual_transactions = actual_transactions.unwrap();
        assert_eq!(actual_transactions, transactions_with_version);
    }
}
