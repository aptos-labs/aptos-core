// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{METADATA_UPLOAD_FAILURE_COUNT, PROCESSED_VERSIONS_COUNT};
use anyhow::{ensure, Context, Result};
use velor_indexer_grpc_utils::{
    cache_operator::CacheOperator,
    compression_util::{FileStoreMetadata, StorageFormat, FILE_ENTRY_TRANSACTION_COUNT},
    config::IndexerGrpcFileStoreConfig,
    counters::{log_grpc_step, IndexerGrpcStep},
    file_store_operator::FileStoreOperator,
    types::RedisUrl,
};
use velor_moving_average::MovingAverage;
use std::time::Duration;
use tracing::debug;

// If the version is ahead of the cache head, retry after a short sleep.
const AHEAD_OF_CACHE_SLEEP_DURATION_IN_MILLIS: u64 = 100;
const SERVICE_TYPE: &str = "file_worker";
const MAX_CONCURRENT_BATCHES: usize = 50;

/// Processor tails the data in cache and stores the data in file store.
pub struct Processor {
    cache_operator: CacheOperator<redis::aio::ConnectionManager>,
    file_store_operator: Box<dyn FileStoreOperator>,
    chain_id: u64,
}

impl Processor {
    pub async fn new(
        redis_main_instance_address: RedisUrl,
        file_store_config: IndexerGrpcFileStoreConfig,
        chain_id: u64,
        enable_cache_compression: bool,
    ) -> Result<Self> {
        let cache_storage_format = if enable_cache_compression {
            StorageFormat::Lz4CompressedProto
        } else {
            StorageFormat::Base64UncompressedProto
        };

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

        let mut file_store_operator: Box<dyn FileStoreOperator> = file_store_config.create();
        file_store_operator.verify_storage_bucket_existence().await;
        let file_store_metadata: Option<FileStoreMetadata> =
            file_store_operator.get_file_store_metadata().await;
        if file_store_metadata.is_none() {
            // If metadata doesn't exist, create and upload it and init file store latest version in cache.
            while file_store_operator
                .update_file_store_metadata_with_timeout(chain_id, 0)
                .await
                .is_err()
            {
                tracing::error!(
                    batch_start_version = 0,
                    service_type = SERVICE_TYPE,
                    "[File worker] Failed to update file store metadata. Retrying."
                );
                std::thread::sleep(std::time::Duration::from_millis(500));
                METADATA_UPLOAD_FAILURE_COUNT.inc();
            }
        }
        // Metadata is guaranteed to exist now
        let metadata = file_store_operator.get_file_store_metadata().await.unwrap();

        ensure!(metadata.chain_id == chain_id, "Chain ID mismatch.");
        let batch_start_version = metadata.version;
        // Cache config in the cache
        cache_operator.cache_setup_if_needed().await?;
        match cache_operator.get_chain_id().await? {
            Some(id) => {
                ensure!(id == chain_id, "Chain ID mismatch.");
            },
            None => {
                cache_operator.set_chain_id(chain_id).await?;
            },
        }
        cache_operator
            .update_file_store_latest_version(batch_start_version)
            .await?;
        Ok(Self {
            cache_operator,
            file_store_operator,
            chain_id,
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
        let chain_id = self.chain_id;

        let metadata = self
            .file_store_operator
            .get_file_store_metadata()
            .await
            .unwrap();
        ensure!(metadata.chain_id == chain_id, "Chain ID mismatch.");

        let mut batch_start_version = metadata.version;

        let mut tps_calculator = MovingAverage::new(10_000);
        loop {
            let latest_loop_time = std::time::Instant::now();
            let cache_worker_latest = self.cache_operator.get_latest_version().await?.unwrap();

            // batches tracks the start version of the batches to fetch. 1000 at the time
            let mut batches = vec![];
            let mut start_version = batch_start_version;
            while start_version + (FILE_ENTRY_TRANSACTION_COUNT) < cache_worker_latest {
                batches.push(start_version);
                start_version += FILE_ENTRY_TRANSACTION_COUNT;
                if batches.len() >= MAX_CONCURRENT_BATCHES {
                    break;
                }
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
            let mut tasks = vec![];

            for start_version in batches {
                let mut cache_operator_clone = self.cache_operator.clone();
                let mut file_store_operator_clone = self.file_store_operator.clone_box();
                let task = tokio::spawn(async move {
                    let fetch_start_time = std::time::Instant::now();
                    let transactions = cache_operator_clone
                        .get_transactions(start_version, FILE_ENTRY_TRANSACTION_COUNT)
                        .await
                        .unwrap();
                    let last_transaction = transactions.last().unwrap().clone();
                    log_grpc_step(
                        SERVICE_TYPE,
                        IndexerGrpcStep::FilestoreFetchTxns,
                        Some(start_version as i64),
                        Some((start_version + FILE_ENTRY_TRANSACTION_COUNT - 1) as i64),
                        None,
                        None,
                        Some(fetch_start_time.elapsed().as_secs_f64()),
                        None,
                        Some(FILE_ENTRY_TRANSACTION_COUNT as i64),
                        None,
                    );
                    for (i, txn) in transactions.iter().enumerate() {
                        assert_eq!(txn.version, start_version + i as u64);
                    }
                    let upload_start_time = std::time::Instant::now();
                    let (start, end) = file_store_operator_clone
                        .upload_transaction_batch(chain_id, transactions)
                        .await
                        .unwrap();
                    log_grpc_step(
                        SERVICE_TYPE,
                        IndexerGrpcStep::FilestoreUploadTxns,
                        Some(start_version as i64),
                        Some((start_version + FILE_ENTRY_TRANSACTION_COUNT - 1) as i64),
                        None,
                        None,
                        Some(upload_start_time.elapsed().as_secs_f64()),
                        None,
                        Some(FILE_ENTRY_TRANSACTION_COUNT as i64),
                        None,
                    );

                    (start, end, last_transaction)
                });
                tasks.push(task);
            }
            let (first_version, last_version, first_version_encoded, last_version_encoded) =
                match futures::future::try_join_all(tasks).await {
                    Ok(mut res) => {
                        // Check for gaps
                        res.sort_by(|a, b| a.0.cmp(&b.0));
                        let mut prev_start = None;
                        let mut prev_end = None;

                        let first_version = res.first().unwrap().0;
                        let last_version = res.last().unwrap().1;
                        let first_version_encoded = res.first().unwrap().2.clone();
                        let last_version_encoded = res.last().unwrap().2.clone();
                        let versions: Vec<u64> = res.iter().map(|x| x.0).collect();
                        for result in res {
                            let start = result.0;
                            let end = result.1;
                            if prev_start.is_none() {
                                prev_start = Some(start);
                                prev_end = Some(end);
                            } else {
                                if prev_end.unwrap() + 1 != start {
                                    tracing::error!(
                                        processed_versions = ?versions,
                                        "[Filestore] Gaps in processing data"
                                    );
                                    panic!("[Filestore] Gaps in processing data");
                                }
                                prev_start = Some(start);
                                prev_end = Some(end);
                            }
                        }

                        (
                            first_version,
                            last_version,
                            first_version_encoded,
                            last_version_encoded,
                        )
                    },
                    Err(err) => panic!("Error processing transaction batches: {:?}", err),
                };

            // update next batch start version
            batch_start_version = last_version + 1;
            assert!(
                batch_start_version % FILE_ENTRY_TRANSACTION_COUNT == 0,
                "[Filestore] Batch must be multiple of 1000"
            );
            let size = last_version - first_version + 1;
            PROCESSED_VERSIONS_COUNT.inc_by(size);
            tps_calculator.tick_now(size);

            // Update filestore metadata. First do it in cache for performance then update metadata file
            let start_metadata_upload_time = std::time::Instant::now();
            self.cache_operator
                .update_file_store_latest_version(batch_start_version)
                .await?;
            while self
                .file_store_operator
                .update_file_store_metadata_with_timeout(chain_id, batch_start_version)
                .await
                .is_err()
            {
                tracing::error!(
                    batch_start_version = batch_start_version,
                    "Failed to update file store metadata. Retrying."
                );
                std::thread::sleep(std::time::Duration::from_millis(500));
                METADATA_UPLOAD_FAILURE_COUNT.inc();
            }
            log_grpc_step(
                SERVICE_TYPE,
                IndexerGrpcStep::FilestoreUpdateMetadata,
                Some(first_version as i64),
                Some(last_version as i64),
                None,
                None,
                Some(start_metadata_upload_time.elapsed().as_secs_f64()),
                None,
                Some(size as i64),
                None,
            );

            let start_version_timestamp = first_version_encoded.timestamp;
            let end_version_timestamp = last_version_encoded.timestamp;
            let full_loop_duration = latest_loop_time.elapsed().as_secs_f64();
            log_grpc_step(
                SERVICE_TYPE,
                IndexerGrpcStep::FilestoreProcessedBatch,
                Some(first_version as i64),
                Some(last_version as i64),
                start_version_timestamp.as_ref(),
                end_version_timestamp.as_ref(),
                Some(full_loop_duration),
                None,
                Some(size as i64),
                None,
            );
        }
    }
}
