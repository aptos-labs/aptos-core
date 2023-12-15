// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{
    ERROR_COUNT, LATEST_PROCESSED_VERSION as LATEST_PROCESSED_VERSION_OLD, PROCESSED_BATCH_SIZE,
    PROCESSED_VERSIONS_COUNT, WAIT_FOR_FILE_STORE_COUNTER,
};
use anyhow::{bail, ensure, Context, Result};
use aptos_indexer_grpc_utils::{
    cache_operator::CacheOperator,
    config::IndexerGrpcFileStoreConfig,
    counters::{log_grpc_step, IndexerGrpcStep},
    create_grpc_client_with_retry,
    file_store_operator::FileStoreOperator,
    types::RedisUrl,
};
use aptos_protos::internal::fullnode::v1::{
    stream_status::StatusType, transactions_from_node_response::Response,
    GetTransactionsFromNodeRequest, TransactionsOutput,
};
use futures::{self, StreamExt};
use prost::Message;
use url::Url;

const FILE_STORE_VERSIONS_RESERVED: u64 = 150_000;
// Cache worker will wait if filestore is behind by
// `FILE_STORE_VERSIONS_RESERVED` versions
// This is pinging the cache so it's OK to be more aggressive
const CACHE_WORKER_WAIT_FOR_FILE_STORE_MS: u64 = 100;
// This is the time we wait for the file store to be ready. It should only be
// kicked off when there's no metadata in the file store.
const FILE_STORE_METADATA_WAIT_MS: u64 = 2000;

const SERVICE_TYPE: &str = "cache_worker";

type ProcessedVersionsAndSize = (usize, usize);

/// TODO: make this static as well
pub struct Worker {
    /// Redis client.
    redis_client: redis::Client,
    /// Fullnode grpc address.
    fullnode_grpc_address: Url,
    /// File store config
    file_store: IndexerGrpcFileStoreConfig,
}

impl Worker {
    pub async fn new(
        fullnode_grpc_address: Url,
        redis_main_instance_address: RedisUrl,
        file_store: IndexerGrpcFileStoreConfig,
    ) -> Result<Self> {
        let redis_client = redis::Client::open(redis_main_instance_address.0.clone())
            .with_context(|| {
                format!(
                    "[Indexer Cache] Failed to create redis client for {}",
                    redis_main_instance_address
                )
            })?;
        Ok(Self {
            redis_client,
            file_store,
            fullnode_grpc_address,
        })
    }

    /// Worker flow looks like this:
    /// 1. Set up and validation
    ///  * Confirm that filestore is ready. If not, wait. Filestore should bootstrap cache as well
    ///  * The starting_version and chain id will be always from filestore metadata
    /// 2. Set up the grpc stream and validate the first response for chain id.
    /// 3. Infinite loop on the rest
    pub async fn run(&mut self) -> Result<()> {
        // Setup cache operator.
        let conn = self
            .redis_client
            .get_tokio_connection_manager()
            .await
            .context("Get redis connection failed.")?;
        let mut cache_operator = CacheOperator::new(conn);

        // Set up file store operator.
        let file_store_operator: Box<dyn FileStoreOperator> = self.file_store.create();
        // This ensures that metadata is created before we start the cache worker.
        let file_store_metadata = loop {
            match file_store_operator.get_file_store_metadata().await {
                Some(metadata) => {
                    // Guaranteed that filestore metadata exists at this point.
                    if cache_operator
                        .get_file_store_latest_version()
                        .await?
                        .is_some()
                    {
                        // Wait until cache populates the file version.
                        break metadata;
                    }
                },
                None => {
                    tracing::warn!(
                        "[Indexer Cache] File store metadata not found. Waiting for {} ms.",
                        FILE_STORE_METADATA_WAIT_MS
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(
                        FILE_STORE_METADATA_WAIT_MS,
                    ))
                    .await;
                },
            }
        };
        let starting_version = file_store_metadata.version;

        // Check cache chain id.
        let cache_chain_id = cache_operator
            .get_chain_id()
            .await?
            .context("Failed to get chain id from cache")?;
        ensure!(cache_chain_id == file_store_metadata.chain_id);

        // TODO: cache worker should restart from file store version.s
        // For now, let's pick the max of cache.starting_version and file_store_metadata
        let cache_latest_version = cache_operator.get_latest_version().await?.unwrap_or(0);
        let starting_version = std::cmp::max(starting_version, cache_latest_version);

        // Now, starts GRPC stream with fullnode
        let mut rpc_client =
            create_grpc_client_with_retry(self.fullnode_grpc_address.clone()).await?;
        let request = tonic::Request::new(GetTransactionsFromNodeRequest {
            starting_version: Some(starting_version),
            ..Default::default()
        });
        let response = rpc_client
            .get_transactions_from_node(request)
            .await
            .with_context(|| {
                format!(
                    "Failed to get transactions from node at starting version {}",
                    starting_version
                )
            })?;
        // Verify that we're talking to the correct fullnode
        let mut resp_stream = response.into_inner();
        let response_item = resp_stream
            .next()
            .await
            .context("Response should not be empty.")??;
        let init_signal = response_item
            .response
            .context("Response should not be empty")?;
        match init_signal {
            Response::Status(status) => {
                ensure!(status.r#type() == StatusType::Init);
                ensure!(status.start_version == starting_version);
                ensure!(response_item.chain_id as u64 == file_store_metadata.chain_id);
            },
            _ => {
                bail!("[Indexer Cache] First response should always be init");
            },
        }

        let mut current_version = starting_version;
        let mut transaction_count = 0;
        let mut starting_time = std::time::Instant::now();
        let mut size_in_bytes = 0;
        // Process the infinite stream.
        while let Some(received) = resp_stream.next().await {
            let response_item = received?;
            let response = response_item
                .response
                .context("Response should not be empty")?;
            match response {
                // This is the end of the batch.
                Response::Status(status) => {
                    let status_type = status.r#type();
                    let response_end_version =
                        status.end_version.expect("End version in BatchEnd signal.");
                    ensure!(status_type == StatusType::BatchEnd);
                    ensure!(response_item.chain_id as u64 == file_store_metadata.chain_id);
                    ensure!(current_version + transaction_count == response_end_version + 1);
                    let batch_start_version = current_version;
                    cache_operator
                        .update_cache_latest_version_with_check(
                            batch_start_version,
                            response_end_version,
                        )
                        .await
                        .context("Failed to update the latest version in the cache")?;
                    log_grpc_step(
                        SERVICE_TYPE,
                        IndexerGrpcStep::CacheWorkerBatchProcessed,
                        Some(batch_start_version as i64),
                        Some(response_end_version as i64),
                        None,
                        None,
                        Some(starting_time.elapsed().as_secs_f64()),
                        Some(size_in_bytes),
                        Some(transaction_count as i64),
                        None,
                    );
                    // Update the number.
                    current_version = response_end_version + 1;
                    transaction_count = 0;
                    starting_time = std::time::Instant::now();
                    size_in_bytes = 0;
                },
                // This is the data response.
                Response::Data(data) => {
                    // Data Processing.
                    let (processed_transaction_count, processed_transaction_size) =
                        process_data_response(data, &mut cache_operator).await?;
                    transaction_count += processed_transaction_count as u64;
                    size_in_bytes += processed_transaction_size;
                },
            }

            // Check if the file store isn't too far away
            loop {
                let file_store_version = cache_operator
                    .get_file_store_latest_version()
                    .await
                    .expect("Failed to get file store latest version")
                    .unwrap_or(0);

                if file_store_version + FILE_STORE_VERSIONS_RESERVED < current_version {
                    tokio::time::sleep(std::time::Duration::from_millis(
                        CACHE_WORKER_WAIT_FOR_FILE_STORE_MS,
                    ))
                    .await;
                    tracing::warn!(
                        current_version = current_version,
                        file_store_version = file_store_version,
                        "[Indexer Cache] File store version is behind current version too much."
                    );
                    WAIT_FOR_FILE_STORE_COUNTER.inc();
                } else {
                    // File store is up to date, continue cache update.
                    break;
                }
            }
        }

        // It's never expect to have a finite stream.
        panic!("Cache worker exited unexpectedly");
    }
}

async fn process_data_response(
    data: TransactionsOutput,
    cache_operator: &mut CacheOperator<redis::aio::ConnectionManager>,
) -> Result<ProcessedVersionsAndSize> {
    let transactions = &data.transactions;
    let starting_time = std::time::Instant::now();
    let transaction_count = transactions.len();
    let first_transaction = transactions
        .first()
        .context("There were unexpectedly no transactions in the response")?;
    let last_transaction = transactions
        .last()
        .context("There were unexpectedly no transactions in the response")?;
    let start_version = first_transaction.version;
    let first_transaction_pb_timestamp = first_transaction.timestamp.clone();
    let last_transaction_pb_timestamp = last_transaction.timestamp.clone();
    let transactions_serialized = transactions
        .iter()
        .map(|tx| {
            let timestamp_in_seconds = match tx.timestamp {
                Some(ref timestamp) => timestamp.seconds as u64,
                None => 0,
            };
            let mut encoded_proto_data = vec![];
            tx.encode(&mut encoded_proto_data)
                .context("Encode transaction failed.")?;
            let base64_encoded_proto_data = base64::encode(encoded_proto_data);
            Ok((tx.version, base64_encoded_proto_data, timestamp_in_seconds))
        })
        .collect::<Result<Vec<(u64, String, u64)>>>()?;
    let size_in_bytes = transactions_serialized
        .iter()
        .fold(0, |acc, (_, tx, _)| acc + tx.len());
    // Old metrics.
    PROCESSED_VERSIONS_COUNT.inc_by(transaction_count as u64);
    LATEST_PROCESSED_VERSION_OLD.set(start_version as i64);
    PROCESSED_BATCH_SIZE.set(transaction_count as i64);
    match cache_operator
        .update_cache_transactions(transactions_serialized)
        .await
    {
        Ok(_) => {
            log_grpc_step(
                SERVICE_TYPE,
                IndexerGrpcStep::CacheWorkerTxnsProcessed,
                Some(first_transaction.version as i64),
                Some(last_transaction.version as i64),
                first_transaction_pb_timestamp.as_ref(),
                last_transaction_pb_timestamp.as_ref(),
                Some(starting_time.elapsed().as_secs_f64()),
                Some(size_in_bytes),
                Some((last_transaction.version - first_transaction.version + 1) as i64),
                None,
            );
        },
        Err(e) => {
            ERROR_COUNT
                .with_label_values(&["failed_to_update_cache_version"])
                .inc();
            bail!("Update cache with version failed: {}", e);
        },
    }
    Ok((transaction_count, size_in_bytes))
}
