// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{
    BYTES_READY_TO_TRANSFER_FROM_SERVER, BYTES_READY_TO_TRANSFER_FROM_SERVER_AFTER_STRIPPING,
    CONNECTION_COUNT, ERROR_COUNT, LATEST_PROCESSED_VERSION_PER_PROCESSOR,
    NUM_TRANSACTIONS_STRIPPED, PROCESSED_LATENCY_IN_SECS_PER_PROCESSOR,
    PROCESSED_VERSIONS_COUNT_PER_PROCESSOR, SHORT_CONNECTION_COUNT,
};
use anyhow::{Context, Result};
use aptos_indexer_grpc_utils::{
    cache_operator::{CacheBatchGetStatus, CacheCoverageStatus, CacheOperator},
    chunk_transactions,
    compression_util::{CacheEntry, StorageFormat},
    config::IndexerGrpcFileStoreConfig,
    constants::{
        IndexerGrpcRequestMetadata, GRPC_AUTH_TOKEN_HEADER, GRPC_REQUEST_NAME_HEADER,
        MESSAGE_SIZE_LIMIT, REQUEST_HEADER_APTOS_APPLICATION_NAME, REQUEST_HEADER_APTOS_EMAIL,
        REQUEST_HEADER_APTOS_IDENTIFIER, REQUEST_HEADER_APTOS_IDENTIFIER_TYPE,
    },
    counters::{log_grpc_step, IndexerGrpcStep, NUM_MULTI_FETCH_OVERLAPPED_VERSIONS},
    file_store_operator::FileStoreOperator,
    in_memory_cache::InMemoryCache,
    time_diff_since_pb_timestamp_in_secs,
    types::RedisUrl,
};
use aptos_moving_average::MovingAverage;
use aptos_protos::{
    indexer::v1::{raw_data_server::RawData, GetTransactionsRequest, TransactionsResponse},
    transaction::v1::{transaction::TxnData, Transaction},
};
use aptos_transaction_filter::{BooleanTransactionFilter, Filterable};
use futures::Stream;
use prost::Message;
use redis::Client;
use std::{
    collections::HashMap,
    pin::Pin,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::{channel, error::SendTimeoutError};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tracing::{error, info, warn};
use uuid::Uuid;

type ResponseStream = Pin<Box<dyn Stream<Item = Result<TransactionsResponse, Status>> + Send>>;

const MOVING_AVERAGE_WINDOW_SIZE: u64 = 10_000;
// When trying to fetch beyond the current head of cache, the server will retry after this duration.
const AHEAD_OF_CACHE_RETRY_SLEEP_DURATION_MS: u64 = 50;
// When error happens when fetching data from cache and file store, the server will retry after this duration.
// TODO(larry): fix all errors treated as transient errors.
const TRANSIENT_DATA_ERROR_RETRY_SLEEP_DURATION_MS: u64 = 1000;
// This is the time we wait for the file store to be ready. It should only be
// kicked off when there's no metadata in the file store.
const FILE_STORE_METADATA_WAIT_MS: u64 = 2000;

// The server will retry to send the response to the client and give up after RESPONSE_CHANNEL_SEND_TIMEOUT.
// This is to prevent the server from being occupied by a slow client.
const RESPONSE_CHANNEL_SEND_TIMEOUT: Duration = Duration::from_secs(120);

const SHORT_CONNECTION_DURATION_IN_SECS: u64 = 10;

const RESPONSE_HEADER_APTOS_CONNECTION_ID_HEADER: &str = "x-aptos-connection-id";
const SERVICE_TYPE: &str = "data_service";

// Number of times to retry fetching a given txn block from the stores
pub const NUM_DATA_FETCH_RETRIES: u8 = 5;

// Max number of tasks to reach out to TXN stores with
const MAX_FETCH_TASKS_PER_REQUEST: u64 = 5;
// The number of transactions we store per txn block; this is used to determine max num of tasks
const TRANSACTIONS_PER_STORAGE_BLOCK: u64 = 1000;

pub struct RawDataServerWrapper {
    pub redis_client: Arc<redis::Client>,
    pub file_store_config: IndexerGrpcFileStoreConfig,
    pub data_service_response_channel_size: usize,
    pub txns_to_strip_filter: BooleanTransactionFilter,
    pub cache_storage_format: StorageFormat,
    in_memory_cache: Arc<InMemoryCache>,
}

// Exclude in_memory-cache
impl std::fmt::Debug for RawDataServerWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawDataServerWrapper")
            .field("redis_client", &"Arc<redis::Client>")
            .field("file_store_config", &self.file_store_config)
            .field(
                "data_service_response_channel_size",
                &self.data_service_response_channel_size,
            )
            .field("txns_to_strip_filter", &self.txns_to_strip_filter)
            .field("cache_storage_format", &self.cache_storage_format)
            .finish()
    }
}

impl RawDataServerWrapper {
    pub fn new(
        redis_address: RedisUrl,
        file_store_config: IndexerGrpcFileStoreConfig,
        data_service_response_channel_size: usize,
        txns_to_strip_filter: BooleanTransactionFilter,
        cache_storage_format: StorageFormat,
        in_memory_cache: Arc<InMemoryCache>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            redis_client: Arc::new(
                redis::Client::open(redis_address.0.clone()).with_context(|| {
                    format!("Failed to create redis client for {}", redis_address)
                })?,
            ),
            file_store_config,
            data_service_response_channel_size,
            txns_to_strip_filter,
            cache_storage_format,
            in_memory_cache,
        })
    }
}

/// Enum to represent the status of the data fetching overall.
enum TransactionsDataStatus {
    // Data fetching is successful.
    Success(Vec<Transaction>),
    // Ahead of current head of cache.
    AheadOfCache,
}

/// RawDataServerWrapper handles the get transactions requests from cache and file store.
#[tonic::async_trait]
impl RawData for RawDataServerWrapper {
    type GetTransactionsStream = ResponseStream;

    /// GetTransactionsStream is a streaming GRPC endpoint:
    /// 1. Fetches data from cache and file store.
    ///    1.1. If the data is beyond the current head of cache, retry after a short sleep.
    ///    1.2. If the data is not in cache, fetch the data from file store.
    ///    1.3. If the data is not in file store, stream connection will break.
    ///    1.4  If error happens, retry after a short sleep.
    /// 2. Push data into channel to stream to the client.
    ///    2.1. If the channel is full, do not fetch and retry after a short sleep.
    async fn get_transactions(
        &self,
        req: Request<GetTransactionsRequest>,
    ) -> Result<Response<Self::GetTransactionsStream>, Status> {
        // Get request identity. The request is already authenticated by the interceptor.
        let request_metadata = match get_request_metadata(&req) {
            Ok(request_metadata) => request_metadata,
            _ => return Result::Err(Status::aborted("Invalid request token")),
        };
        CONNECTION_COUNT
            .with_label_values(&request_metadata.get_label_values())
            .inc();
        let request = req.into_inner();

        let transactions_count = request.transactions_count;

        // Response channel to stream the data to the client.
        let (tx, rx) = channel(self.data_service_response_channel_size);
        let current_version = match &request.starting_version {
            Some(version) => *version,
            // Live mode if starting version isn't specified
            None => self
                .in_memory_cache
                .latest_version()
                .await
                .saturating_sub(1),
        };

        let file_store_operator: Box<dyn FileStoreOperator> = self.file_store_config.create();
        let file_store_operator = Arc::new(file_store_operator);

        // Adds tracing context for the request.
        log_grpc_step(
            SERVICE_TYPE,
            IndexerGrpcStep::DataServiceNewRequestReceived,
            Some(current_version as i64),
            transactions_count.map(|v| (v as i64 + current_version as i64 - 1)),
            None,
            None,
            None,
            None,
            None,
            Some(&request_metadata),
        );

        let redis_client = self.redis_client.clone();
        let cache_storage_format = self.cache_storage_format;
        let request_metadata = Arc::new(request_metadata);
        let txns_to_strip_filter = self.txns_to_strip_filter.clone();
        let in_memory_cache = self.in_memory_cache.clone();
        tokio::spawn({
            let request_metadata = request_metadata.clone();
            async move {
                data_fetcher_task(
                    redis_client,
                    file_store_operator,
                    cache_storage_format,
                    request_metadata,
                    transactions_count,
                    tx,
                    txns_to_strip_filter,
                    current_version,
                    in_memory_cache,
                )
                .await;
            }
        });

        let output_stream = ReceiverStream::new(rx);
        let mut response = Response::new(Box::pin(output_stream) as Self::GetTransactionsStream);

        response.metadata_mut().insert(
            RESPONSE_HEADER_APTOS_CONNECTION_ID_HEADER,
            tonic::metadata::MetadataValue::from_str(&request_metadata.request_connection_id)
                .unwrap(),
        );
        Ok(response)
    }
}

enum DataFetchSubTaskResult {
    BatchSuccess(Vec<Vec<Transaction>>),
    Success(Vec<Transaction>),
    NoResults,
}

async fn get_data_with_tasks(
    start_version: u64,
    transactions_count: Option<u64>,
    chain_id: u64,
    cache_operator: &mut CacheOperator<redis::aio::ConnectionManager>,
    file_store_operator: Arc<Box<dyn FileStoreOperator>>,
    request_metadata: Arc<IndexerGrpcRequestMetadata>,
    cache_storage_format: StorageFormat,
    in_memory_cache: Arc<InMemoryCache>,
) -> DataFetchSubTaskResult {
    let start_time = Instant::now();
    let in_memory_transactions = in_memory_cache.get_transactions(start_version).await;
    if !in_memory_transactions.is_empty() {
        log_grpc_step(
            SERVICE_TYPE,
            IndexerGrpcStep::DataServiceFetchingDataFromInMemoryCache,
            Some(start_version as i64),
            Some(in_memory_transactions.last().as_ref().unwrap().version as i64),
            None,
            None,
            Some(start_time.elapsed().as_secs_f64()),
            None,
            Some(in_memory_transactions.len() as i64),
            Some(&request_metadata),
        );
        return DataFetchSubTaskResult::BatchSuccess(chunk_transactions(
            in_memory_transactions,
            MESSAGE_SIZE_LIMIT,
        ));
    }
    let cache_coverage_status = cache_operator
        .check_cache_coverage_status(start_version)
        .await;

    let num_tasks_to_use = match cache_coverage_status {
        Ok(CacheCoverageStatus::DataNotReady) => return DataFetchSubTaskResult::NoResults,
        Ok(CacheCoverageStatus::CacheHit(_)) => 1,
        Ok(CacheCoverageStatus::CacheEvicted) => match transactions_count {
            None => MAX_FETCH_TASKS_PER_REQUEST,
            Some(transactions_count) => {
                let num_tasks = transactions_count / TRANSACTIONS_PER_STORAGE_BLOCK;
                if num_tasks >= MAX_FETCH_TASKS_PER_REQUEST {
                    // Limit the max tasks to MAX_FETCH_TASKS_PER_REQUEST
                    MAX_FETCH_TASKS_PER_REQUEST
                } else if num_tasks < 1 {
                    // Limit the min tasks to 1
                    1
                } else {
                    num_tasks
                }
            },
        },
        Err(_) => {
            error!("[Data Service] Failed to get cache coverage status.");
            panic!("Failed to get cache coverage status.");
        },
    };

    let mut tasks = tokio::task::JoinSet::new();
    let mut current_version = start_version;

    for _ in 0..num_tasks_to_use {
        tasks.spawn({
            // TODO: arc this instead of cloning
            let mut cache_operator = cache_operator.clone();
            let file_store_operator = file_store_operator.clone();
            let request_metadata = request_metadata.clone();
            async move {
                get_data_in_task(
                    current_version,
                    chain_id,
                    &mut cache_operator,
                    file_store_operator,
                    request_metadata.clone(),
                    cache_storage_format,
                )
                .await
            }
        });
        // Storage is in block of 1000: we align our current version fetch to the nearest block
        current_version += TRANSACTIONS_PER_STORAGE_BLOCK;
        current_version -= current_version % TRANSACTIONS_PER_STORAGE_BLOCK;
    }

    let mut transactions: Vec<Vec<Transaction>> = vec![];
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(DataFetchSubTaskResult::Success(txns)) => {
                transactions.push(txns);
            },
            Ok(DataFetchSubTaskResult::NoResults) => {},
            Err(e) => {
                error!(
                    error = e.to_string(),
                    "[Data Service] Failed to get data from cache and file store."
                );
                panic!("Failed to get data from cache and file store.");
            },
            Ok(_) => unreachable!("Fetching from a single task will never return a batch"),
        }
    }

    if transactions.is_empty() {
        DataFetchSubTaskResult::NoResults
    } else {
        DataFetchSubTaskResult::BatchSuccess(transactions)
    }
}

async fn get_data_in_task(
    start_version: u64,
    chain_id: u64,
    cache_operator: &mut CacheOperator<redis::aio::ConnectionManager>,
    file_store_operator: Arc<Box<dyn FileStoreOperator>>,
    request_metadata: Arc<IndexerGrpcRequestMetadata>,
    cache_storage_format: StorageFormat,
) -> DataFetchSubTaskResult {
    let current_batch_start_time = std::time::Instant::now();

    let fetched = data_fetch(
        start_version,
        cache_operator,
        file_store_operator,
        request_metadata.clone(),
        cache_storage_format,
    );

    let transaction_data = match fetched.await {
        Ok(TransactionsDataStatus::Success(transactions)) => transactions,
        Ok(TransactionsDataStatus::AheadOfCache) => {
            info!(
                start_version = start_version,
                request_identifier = request_metadata.request_identifier.as_str(),
                processor_name = request_metadata.processor_name.as_str(),
                connection_id = request_metadata.request_connection_id.as_str(),
                duration_in_secs = current_batch_start_time.elapsed().as_secs_f64(),
                service_type = SERVICE_TYPE,
                "[Data Service] Requested data is ahead of cache. Sleeping for {} ms.",
                AHEAD_OF_CACHE_RETRY_SLEEP_DURATION_MS,
            );
            ahead_of_cache_data_handling().await;
            // Retry after a short sleep.
            return DataFetchSubTaskResult::NoResults;
        },
        Err(e) => {
            ERROR_COUNT.with_label_values(&["data_fetch_failed"]).inc();
            data_fetch_error_handling(e, start_version, chain_id).await;
            // Retry after a short sleep.
            return DataFetchSubTaskResult::NoResults;
        },
    };
    DataFetchSubTaskResult::Success(transaction_data)
}

// This is a task spawned off for servicing a users' request
async fn data_fetcher_task(
    redis_client: Arc<Client>,
    file_store_operator: Arc<Box<dyn FileStoreOperator>>,
    cache_storage_format: StorageFormat,
    request_metadata: Arc<IndexerGrpcRequestMetadata>,
    transactions_count: Option<u64>,
    tx: tokio::sync::mpsc::Sender<Result<TransactionsResponse, Status>>,
    txns_to_strip_filter: BooleanTransactionFilter,
    mut current_version: u64,
    in_memory_cache: Arc<InMemoryCache>,
) {
    let mut connection_start_time = Some(std::time::Instant::now());
    let mut transactions_count = transactions_count;

    // Establish redis connection
    let conn = match redis_client.get_tokio_connection_manager().await {
        Ok(conn) => conn,
        Err(e) => {
            ERROR_COUNT
                .with_label_values(&["redis_connection_failed"])
                .inc();
            // Connection will be dropped anyway, so we ignore the error here.
            let _result = tx
                .send_timeout(
                    Err(Status::unavailable(
                        "[Data Service] Cannot connect to Redis; please retry.",
                    )),
                    RESPONSE_CHANNEL_SEND_TIMEOUT,
                )
                .await;
            error!(
                error = e.to_string(),
                "[Data Service] Failed to get redis connection."
            );
            return;
        },
    };
    let mut cache_operator = CacheOperator::new(conn, cache_storage_format);

    // Validate chain id
    let mut metadata = file_store_operator.get_file_store_metadata().await;
    while metadata.is_none() {
        metadata = file_store_operator.get_file_store_metadata().await;
        tracing::warn!(
            "[File worker] File store metadata not found. Waiting for {} ms.",
            FILE_STORE_METADATA_WAIT_MS
        );
        tokio::time::sleep(std::time::Duration::from_millis(
            FILE_STORE_METADATA_WAIT_MS,
        ))
        .await;
    }

    let metadata_chain_id = metadata.unwrap().chain_id;

    // Validate redis chain id. Must be present by the time it gets here
    let chain_id = match cache_operator.get_chain_id().await {
        Ok(chain_id) => chain_id.unwrap(),
        Err(e) => {
            ERROR_COUNT
                .with_label_values(&["redis_get_chain_id_failed"])
                .inc();
            // Connection will be dropped anyway, so we ignore the error here.
            let _result = tx
                .send_timeout(
                    Err(Status::unavailable(
                        "[Data Service] Cannot get the chain id from redis; please retry.",
                    )),
                    RESPONSE_CHANNEL_SEND_TIMEOUT,
                )
                .await;
            error!(
                error = e.to_string(),
                "[Data Service] Failed to get chain id from redis."
            );
            return;
        },
    };

    if metadata_chain_id != chain_id {
        let _result = tx
            .send_timeout(
                Err(Status::unavailable("[Data Service] Chain ID mismatch.")),
                RESPONSE_CHANNEL_SEND_TIMEOUT,
            )
            .await;
        error!("[Data Service] Chain ID mismatch.",);
        return;
    }

    // Data service metrics.
    let mut tps_calculator = MovingAverage::new(MOVING_AVERAGE_WINDOW_SIZE);

    loop {
        // 1. Fetch data from cache and file store.
        let transaction_data = match get_data_with_tasks(
            current_version,
            transactions_count,
            chain_id,
            &mut cache_operator,
            file_store_operator.clone(),
            request_metadata.clone(),
            cache_storage_format,
            in_memory_cache.clone(),
        )
        .await
        {
            DataFetchSubTaskResult::BatchSuccess(txns) => txns,
            DataFetchSubTaskResult::Success(_) => {
                unreachable!("Fetching from multiple tasks will never return a single vector")
            },
            DataFetchSubTaskResult::NoResults => continue,
        };

        let mut transaction_data = ensure_sequential_transactions(transaction_data);

        // TODO: Unify the truncation logic for start and end.
        if let Some(count) = transactions_count {
            if count == 0 {
                // End the data stream.
                // Since the client receives all the data it requested, we don't count it as a short connection.
                connection_start_time = None;
                break;
            } else if (count as usize) < transaction_data.len() {
                // Trim the data to the requested end version.
                transaction_data.truncate(count as usize);
                transactions_count = Some(0);
            } else {
                transactions_count = Some(count - transaction_data.len() as u64);
            }
        };
        // Note: this is the protobuf encoded transaction size.
        let bytes_ready_to_transfer = transaction_data
            .iter()
            .map(|t| t.encoded_len())
            .sum::<usize>();
        BYTES_READY_TO_TRANSFER_FROM_SERVER
            .with_label_values(&request_metadata.get_label_values())
            .inc_by(bytes_ready_to_transfer as u64);
        // 2. Push the data to the response channel, i.e. stream the data to the client.
        let current_batch_size = transaction_data.as_slice().len();
        let end_of_batch_version = transaction_data.as_slice().last().unwrap().version;
        let (resp_items, num_stripped) = get_transactions_responses_builder(
            transaction_data,
            chain_id as u32,
            &txns_to_strip_filter,
        );
        NUM_TRANSACTIONS_STRIPPED
            .with_label_values(&request_metadata.get_label_values())
            .inc_by(num_stripped as u64);
        let bytes_ready_to_transfer_after_stripping = resp_items
            .iter()
            .flat_map(|response| &response.transactions)
            .map(|t| t.encoded_len())
            .sum::<usize>();
        BYTES_READY_TO_TRANSFER_FROM_SERVER_AFTER_STRIPPING
            .with_label_values(&request_metadata.get_label_values())
            .inc_by(bytes_ready_to_transfer_after_stripping as u64);
        let data_latency_in_secs = resp_items
            .last()
            .unwrap()
            .transactions
            .last()
            .unwrap()
            .timestamp
            .as_ref()
            .map(time_diff_since_pb_timestamp_in_secs);

        match channel_send_multiple_with_timeout(resp_items, tx.clone(), request_metadata.clone())
            .await
        {
            Ok(_) => {
                // TODO: Reasses whether this metric is useful.
                LATEST_PROCESSED_VERSION_PER_PROCESSOR
                    .with_label_values(&request_metadata.get_label_values())
                    .set(end_of_batch_version as i64);
                PROCESSED_VERSIONS_COUNT_PER_PROCESSOR
                    .with_label_values(&request_metadata.get_label_values())
                    .inc_by(current_batch_size as u64);
                if let Some(data_latency_in_secs) = data_latency_in_secs {
                    PROCESSED_LATENCY_IN_SECS_PER_PROCESSOR
                        .with_label_values(&request_metadata.get_label_values())
                        .set(data_latency_in_secs);
                }
            },
            Err(SendTimeoutError::Timeout(_)) => {
                warn!("[Data Service] Receiver is full; exiting.");
                break;
            },
            Err(SendTimeoutError::Closed(_)) => {
                warn!("[Data Service] Receiver is closed; exiting.");
                break;
            },
        }
        // 3. Update the current version and record current tps.
        tps_calculator.tick_now(current_batch_size as u64);
        current_version = end_of_batch_version + 1;
    }
    info!(
        request_identifier = request_metadata.request_identifier.as_str(),
        processor_name = request_metadata.processor_name.as_str(),
        connection_id = request_metadata.request_connection_id.as_str(),
        service_type = SERVICE_TYPE,
        "[Data Service] Client disconnected."
    );
    if let Some(start_time) = connection_start_time {
        if start_time.elapsed().as_secs() < SHORT_CONNECTION_DURATION_IN_SECS {
            SHORT_CONNECTION_COUNT
                .with_label_values(&request_metadata.get_label_values())
                .inc();
        }
    }
}

/// Takes in multiple batches of transactions, and:
/// 1. De-dupes in the case of overlap (but log to prom metric)
/// 2. Panics in cases of gaps
fn ensure_sequential_transactions(mut batches: Vec<Vec<Transaction>>) -> Vec<Transaction> {
    // If there's only one, no sorting required
    if batches.len() == 1 {
        return batches.pop().unwrap();
    }

    // Sort by the first version per batch, ascending
    batches.sort_by(|a, b| a.first().unwrap().version.cmp(&b.first().unwrap().version));
    let first_version = batches.first().unwrap().first().unwrap().version;
    let last_version = batches.last().unwrap().last().unwrap().version;
    let mut transactions: Vec<Transaction> = vec![];

    let mut prev_start = None;
    let mut prev_end = None;
    for mut batch in batches {
        let mut start_version = batch.first().unwrap().version;
        let end_version = batch.last().unwrap().version;
        if prev_start.is_some() {
            let prev_start = prev_start.unwrap();
            let prev_end = prev_end.unwrap();
            // If this batch is fully contained within the previous batch, skip it
            if prev_start <= start_version && prev_end >= end_version {
                NUM_MULTI_FETCH_OVERLAPPED_VERSIONS
                    .with_label_values(&[SERVICE_TYPE, "full"])
                    .inc_by(end_version - start_version);
                continue;
            }
            // If this batch overlaps with the previous batch, combine them
            if prev_end >= start_version {
                NUM_MULTI_FETCH_OVERLAPPED_VERSIONS
                    .with_label_values(&[SERVICE_TYPE, "partial"])
                    .inc_by(prev_end - start_version + 1);
                tracing::debug!(
                    batch_first_version = first_version,
                    batch_last_version = last_version,
                    start_version = start_version,
                    end_version = end_version,
                    prev_start = ?prev_start,
                    prev_end = prev_end,
                    "[Filestore] Overlapping version data"
                );
                batch.drain(0..(prev_end - start_version + 1) as usize);
                start_version = batch.first().unwrap().version;
            }

            // Otherwise there is a gap
            if prev_end + 1 != start_version {
                NUM_MULTI_FETCH_OVERLAPPED_VERSIONS
                    .with_label_values(&[SERVICE_TYPE, "gap"])
                    .inc_by(prev_end - start_version + 1);

                tracing::error!(
                    batch_first_version = first_version,
                    batch_last_version = last_version,
                    start_version = start_version,
                    end_version = end_version,
                    prev_start = ?prev_start,
                    prev_end = prev_end,
                    "[Filestore] Gaps or dupes in processing version data"
                );
                panic!("[Filestore] Gaps in processing data batch_first_version: {}, batch_last_version: {}, start_version: {}, end_version: {}, prev_start: {:?}, prev_end: {:?}",
                       first_version,
                       last_version,
                       start_version,
                       end_version,
                       prev_start,
                       prev_end,
                );
            }
        }

        prev_start = Some(start_version);
        prev_end = Some(end_version);
        transactions.extend(batch);
    }

    transactions
}

/// Builds the response for the get transactions request. Partial batch is ok, i.e., a
/// batch with transactions < 1000.
///
/// It also returns the number of txns that were stripped.
fn get_transactions_responses_builder(
    transactions: Vec<Transaction>,
    chain_id: u32,
    txns_to_strip_filter: &BooleanTransactionFilter,
) -> (Vec<TransactionsResponse>, usize) {
    let (stripped_transactions, num_stripped) =
        strip_transactions(transactions, txns_to_strip_filter);
    let chunks = chunk_transactions(stripped_transactions, MESSAGE_SIZE_LIMIT);
    let responses = chunks
        .into_iter()
        .map(|chunk| TransactionsResponse {
            chain_id: Some(chain_id as u64),
            transactions: chunk,
            processed_range: None,
        })
        .collect();
    (responses, num_stripped)
}

// This is a CPU bound operation, so we spawn_blocking
async fn deserialize_cached_transactions(
    transactions: Vec<Vec<u8>>,
    storage_format: StorageFormat,
) -> anyhow::Result<Vec<Transaction>> {
    let task = tokio::task::spawn_blocking(move || {
        transactions
            .into_iter()
            .map(|transaction| {
                let cache_entry = CacheEntry::new(transaction, storage_format);
                cache_entry.into_transaction()
            })
            .collect::<Vec<Transaction>>()
    })
    .await;
    task.context("Transaction bytes to CacheEntry deserialization task failed")
}

/// Fetches data from cache or the file store. It returns the data if it is ready in the cache or file store.
/// Otherwise, it returns the status of the data fetching.
async fn data_fetch(
    starting_version: u64,
    cache_operator: &mut CacheOperator<redis::aio::ConnectionManager>,
    file_store_operator: Arc<Box<dyn FileStoreOperator>>,
    request_metadata: Arc<IndexerGrpcRequestMetadata>,
    storage_format: StorageFormat,
) -> anyhow::Result<TransactionsDataStatus> {
    let current_batch_start_time = std::time::Instant::now();
    let batch_get_result = cache_operator
        .batch_get_encoded_proto_data(starting_version)
        .await;

    match batch_get_result {
        // Data is not ready yet in the cache.
        Ok(CacheBatchGetStatus::NotReady) => Ok(TransactionsDataStatus::AheadOfCache),
        Ok(CacheBatchGetStatus::Ok(transactions)) => {
            let decoding_start_time = std::time::Instant::now();
            let size_in_bytes = transactions
                .iter()
                .map(|transaction| transaction.len())
                .sum::<usize>();
            let num_of_transactions = transactions.len();
            let duration_in_secs = current_batch_start_time.elapsed().as_secs_f64();

            let transactions =
                deserialize_cached_transactions(transactions, storage_format).await?;
            let start_version_timestamp = transactions.first().unwrap().timestamp.as_ref();
            let end_version_timestamp = transactions.last().unwrap().timestamp.as_ref();

            log_grpc_step(
                SERVICE_TYPE,
                IndexerGrpcStep::DataServiceDataFetchedCache,
                Some(starting_version as i64),
                Some(starting_version as i64 + num_of_transactions as i64 - 1),
                start_version_timestamp,
                end_version_timestamp,
                Some(duration_in_secs),
                Some(size_in_bytes),
                Some(num_of_transactions as i64),
                Some(&request_metadata),
            );
            log_grpc_step(
                SERVICE_TYPE,
                IndexerGrpcStep::DataServiceTxnsDecoded,
                Some(starting_version as i64),
                Some(starting_version as i64 + num_of_transactions as i64 - 1),
                start_version_timestamp,
                end_version_timestamp,
                Some(decoding_start_time.elapsed().as_secs_f64()),
                Some(size_in_bytes),
                Some(num_of_transactions as i64),
                Some(&request_metadata),
            );

            Ok(TransactionsDataStatus::Success(transactions))
        },
        Ok(CacheBatchGetStatus::EvictedFromCache) => {
            let transactions =
                data_fetch_from_filestore(starting_version, file_store_operator, request_metadata)
                    .await?;
            Ok(TransactionsDataStatus::Success(transactions))
        },
        Err(e) => Err(e),
    }
}

async fn data_fetch_from_filestore(
    starting_version: u64,
    file_store_operator: Arc<Box<dyn FileStoreOperator>>,
    request_metadata: Arc<IndexerGrpcRequestMetadata>,
) -> anyhow::Result<Vec<Transaction>> {
    // Data is evicted from the cache. Fetch from file store.
    let (transactions, io_duration, decoding_duration) = file_store_operator
        .get_transactions_with_durations(starting_version, NUM_DATA_FETCH_RETRIES)
        .await?;
    let size_in_bytes = transactions
        .iter()
        .map(|transaction| transaction.encoded_len())
        .sum::<usize>();
    let num_of_transactions = transactions.len();
    let start_version_timestamp = transactions.first().unwrap().timestamp.as_ref();
    let end_version_timestamp = transactions.last().unwrap().timestamp.as_ref();
    log_grpc_step(
        SERVICE_TYPE,
        IndexerGrpcStep::DataServiceDataFetchedFilestore,
        Some(starting_version as i64),
        Some(starting_version as i64 + num_of_transactions as i64 - 1),
        start_version_timestamp,
        end_version_timestamp,
        Some(io_duration),
        Some(size_in_bytes),
        Some(num_of_transactions as i64),
        Some(&request_metadata),
    );
    log_grpc_step(
        SERVICE_TYPE,
        IndexerGrpcStep::DataServiceTxnsDecoded,
        Some(starting_version as i64),
        Some(starting_version as i64 + num_of_transactions as i64 - 1),
        start_version_timestamp,
        end_version_timestamp,
        Some(decoding_duration),
        Some(size_in_bytes),
        Some(num_of_transactions as i64),
        Some(&request_metadata),
    );
    Ok(transactions)
}

/// Handles the case when the data is not ready in the cache, i.e., beyond the current head.
async fn ahead_of_cache_data_handling() {
    // TODO: add exponential backoff.
    tokio::time::sleep(Duration::from_millis(
        AHEAD_OF_CACHE_RETRY_SLEEP_DURATION_MS,
    ))
    .await;
}

/// Handles data fetch errors, including cache and file store related errors.
async fn data_fetch_error_handling(err: anyhow::Error, current_version: u64, chain_id: u64) {
    error!(
        chain_id = chain_id,
        current_version = current_version,
        "[Data Service] Failed to fetch data from cache and file store. {:?}",
        err
    );
    tokio::time::sleep(Duration::from_millis(
        TRANSIENT_DATA_ERROR_RETRY_SLEEP_DURATION_MS,
    ))
    .await;
}

/// Gets the request metadata. Useful for logging.
fn get_request_metadata(
    req: &Request<GetTransactionsRequest>,
) -> tonic::Result<IndexerGrpcRequestMetadata> {
    let request_metadata_pairs = vec![
        (
            "request_identifier_type",
            REQUEST_HEADER_APTOS_IDENTIFIER_TYPE,
        ),
        ("request_identifier", REQUEST_HEADER_APTOS_IDENTIFIER),
        ("request_email", REQUEST_HEADER_APTOS_EMAIL),
        (
            "request_application_name",
            REQUEST_HEADER_APTOS_APPLICATION_NAME,
        ),
        ("request_token", GRPC_AUTH_TOKEN_HEADER),
        ("processor_name", GRPC_REQUEST_NAME_HEADER),
    ];
    let mut request_metadata_map: HashMap<String, String> = request_metadata_pairs
        .into_iter()
        .map(|(key, value)| {
            (
                key.to_string(),
                req.metadata()
                    .get(value)
                    .map(|value| value.to_str().unwrap_or("unspecified").to_string())
                    .unwrap_or("unspecified".to_string()),
            )
        })
        .collect();
    request_metadata_map.insert(
        "request_connection_id".to_string(),
        Uuid::new_v4().to_string(),
    );
    let request_metadata: IndexerGrpcRequestMetadata =
        serde_json::from_str(&serde_json::to_string(&request_metadata_map).unwrap()).unwrap();
    // TODO: update the request name if these are internal requests.
    Ok(request_metadata)
}

async fn channel_send_multiple_with_timeout(
    resp_items: Vec<TransactionsResponse>,
    tx: tokio::sync::mpsc::Sender<Result<TransactionsResponse, Status>>,
    request_metadata: Arc<IndexerGrpcRequestMetadata>,
) -> Result<(), SendTimeoutError<Result<TransactionsResponse, Status>>> {
    let overall_send_start_time = Instant::now();
    let overall_size_in_bytes = resp_items
        .iter()
        .map(|resp_item| resp_item.encoded_len())
        .sum::<usize>();
    let overall_start_txn = resp_items.first().unwrap().transactions.first().unwrap();
    let overall_end_txn = resp_items.last().unwrap().transactions.last().unwrap();
    let overall_start_version = overall_start_txn.version;
    let overall_end_version = overall_end_txn.version;
    let overall_start_txn_timestamp = overall_start_txn.clone().timestamp;
    let overall_end_txn_timestamp = overall_end_txn.clone().timestamp;

    for resp_item in resp_items {
        let send_start_time = Instant::now();
        let response_size = resp_item.encoded_len();
        let num_of_transactions = resp_item.transactions.len();
        let start_version = resp_item.transactions.first().unwrap().version;
        let end_version = resp_item.transactions.last().unwrap().version;
        let start_version_txn_timestamp = resp_item
            .transactions
            .first()
            .unwrap()
            .timestamp
            .as_ref()
            .unwrap();
        let end_version_txn_timestamp = resp_item
            .transactions
            .last()
            .unwrap()
            .timestamp
            .as_ref()
            .unwrap();

        tx.send_timeout(
            Result::<TransactionsResponse, Status>::Ok(resp_item.clone()),
            RESPONSE_CHANNEL_SEND_TIMEOUT,
        )
        .await?;

        log_grpc_step(
            SERVICE_TYPE,
            IndexerGrpcStep::DataServiceChunkSent,
            Some(start_version as i64),
            Some(end_version as i64),
            Some(start_version_txn_timestamp),
            Some(end_version_txn_timestamp),
            Some(send_start_time.elapsed().as_secs_f64()),
            Some(response_size),
            Some(num_of_transactions as i64),
            Some(&request_metadata),
        );
    }

    log_grpc_step(
        SERVICE_TYPE,
        IndexerGrpcStep::DataServiceAllChunksSent,
        Some(overall_start_version as i64),
        Some(overall_end_version as i64),
        overall_start_txn_timestamp.as_ref(),
        overall_end_txn_timestamp.as_ref(),
        Some(overall_send_start_time.elapsed().as_secs_f64()),
        Some(overall_size_in_bytes),
        Some((overall_end_version - overall_start_version + 1) as i64),
        Some(&request_metadata),
    );

    Ok(())
}

/// This function strips transactions that match the given filter. Stripping means we
/// remove the payload, signature, events, and writesets. Note, the filter can be
/// composed of many conditions, see `BooleanTransactionFilter` for more.
///
/// This returns the mutated txns and the number of txns that were stripped.
fn strip_transactions(
    transactions: Vec<Transaction>,
    txns_to_strip_filter: &BooleanTransactionFilter,
) -> (Vec<Transaction>, usize) {
    let mut stripped_count = 0;

    let stripped_transactions: Vec<Transaction> = transactions
        .into_iter()
        .map(|mut txn| {
            // Note: `is_allowed` means the txn matches the filter, in which case
            // we strip it.
            if txns_to_strip_filter.matches(&txn) {
                stripped_count += 1;
                if let Some(info) = txn.info.as_mut() {
                    info.changes = vec![];
                }
                if let Some(TxnData::User(user_transaction)) = txn.txn_data.as_mut() {
                    user_transaction.events = vec![];
                    if let Some(utr) = user_transaction.request.as_mut() {
                        // Wipe the payload and signature.
                        utr.payload = None;
                        utr.signature = None;
                    }
                }
            }
            txn
        })
        .collect();

    (stripped_transactions, stripped_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_protos::transaction::v1::{
        transaction::TxnData, transaction_payload::Payload, EntryFunctionId, EntryFunctionPayload,
        Event, MoveModuleId, Signature, Transaction, TransactionInfo, TransactionPayload,
        UserTransaction, UserTransactionRequest, WriteSetChange,
    };
    use aptos_transaction_filter::{
        boolean_transaction_filter::APIFilter, filters::UserTransactionFilterBuilder,
        EntryFunctionFilterBuilder, UserTransactionPayloadFilterBuilder,
    };

    fn create_test_transaction(
        module_address: String,
        module_name: String,
        function_name: String,
    ) -> Transaction {
        Transaction {
            version: 1,
            txn_data: Some(TxnData::User(UserTransaction {
                request: Some(UserTransactionRequest {
                    payload: Some(TransactionPayload {
                        r#type: 1,
                        payload: Some(Payload::EntryFunctionPayload(EntryFunctionPayload {
                            function: Some(EntryFunctionId {
                                module: Some(MoveModuleId {
                                    address: module_address,
                                    name: module_name,
                                }),
                                name: function_name,
                            }),
                            ..Default::default()
                        })),
                        // TODO: Try out other types of payloads
                        extra_config: None,
                    }),
                    signature: Some(Signature::default()),
                    ..Default::default()
                }),
                events: vec![Event::default()],
            })),
            info: Some(TransactionInfo {
                changes: vec![WriteSetChange::default()],
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn test_ensure_sequential_transactions_merges_and_sorts() {
        let transactions1 = (1..5)
            .map(|i| Transaction {
                version: i,
                ..Default::default()
            })
            .collect();
        let transactions2 = (5..10)
            .map(|i| Transaction {
                version: i,
                ..Default::default()
            })
            .collect();
        // No overlap, just normal fetching flow
        let transactions1 = ensure_sequential_transactions(vec![transactions1, transactions2]);
        assert_eq!(transactions1.len(), 9);
        assert_eq!(transactions1.first().unwrap().version, 1);
        assert_eq!(transactions1.last().unwrap().version, 9);

        // This is a full overlap
        let transactions2 = (5..7)
            .map(|i| Transaction {
                version: i,
                ..Default::default()
            })
            .collect();
        let transactions1 = ensure_sequential_transactions(vec![transactions1, transactions2]);
        assert_eq!(transactions1.len(), 9);
        assert_eq!(transactions1.first().unwrap().version, 1);
        assert_eq!(transactions1.last().unwrap().version, 9);

        // Partial overlap
        let transactions2 = (5..12)
            .map(|i| Transaction {
                version: i,
                ..Default::default()
            })
            .collect();
        let transactions1 = ensure_sequential_transactions(vec![transactions1, transactions2]);
        assert_eq!(transactions1.len(), 11);
        assert_eq!(transactions1.first().unwrap().version, 1);
        assert_eq!(transactions1.last().unwrap().version, 11);
    }

    const MODULE_ADDRESS: &str = "0x1234";
    const MODULE_NAME: &str = "module";
    const FUNCTION_NAME: &str = "function";

    #[test]
    fn test_transactions_are_stripped_correctly_sender_addresses() {
        let sender_address = "0x1234".to_string();
        // Create a transaction with a user transaction
        let txn = Transaction {
            version: 1,
            txn_data: Some(TxnData::User(UserTransaction {
                request: Some(UserTransactionRequest {
                    sender: sender_address.clone(),
                    payload: Some(TransactionPayload::default()),
                    signature: Some(Signature::default()),
                    ..Default::default()
                }),
                events: vec![Event::default()],
            })),
            info: Some(TransactionInfo {
                changes: vec![WriteSetChange::default()],
                ..Default::default()
            }),
            ..Default::default()
        };

        // Create filter for senders to ignore.
        let sender_filters = vec![sender_address]
            .into_iter()
            .map(|address| {
                BooleanTransactionFilter::from(APIFilter::UserTransactionFilter(
                    UserTransactionFilterBuilder::default()
                        .sender(address)
                        .build()
                        .unwrap(),
                ))
            })
            .collect();
        let filter = BooleanTransactionFilter::new_or(sender_filters);

        let (filtered_txns, num_stripped) = strip_transactions(vec![txn], &filter);
        assert_eq!(num_stripped, 1);
        assert_eq!(filtered_txns.len(), 1);
        let txn = filtered_txns.first().unwrap();
        let user_transaction = match &txn.txn_data {
            Some(TxnData::User(user_transaction)) => user_transaction,
            _ => panic!("Expected user transaction"),
        };
        assert_eq!(user_transaction.request.as_ref().unwrap().payload, None);
        assert_eq!(user_transaction.request.as_ref().unwrap().signature, None);
        assert_eq!(user_transaction.events.len(), 0);
        assert_eq!(txn.info.as_ref().unwrap().changes.len(), 0);
    }

    #[test]
    fn test_transactions_are_stripped_correctly_module_address() {
        let txn = create_test_transaction(
            MODULE_ADDRESS.to_string(),
            MODULE_NAME.to_string(),
            FUNCTION_NAME.to_string(),
        );
        // Testing filter with only address set
        let filter = BooleanTransactionFilter::new_or(vec![BooleanTransactionFilter::from(
            APIFilter::UserTransactionFilter(
                UserTransactionFilterBuilder::default()
                    .payload(
                        UserTransactionPayloadFilterBuilder::default()
                            .function(
                                EntryFunctionFilterBuilder::default()
                                    .address(MODULE_ADDRESS.to_string())
                                    .build()
                                    .unwrap(),
                            )
                            .build()
                            .unwrap(),
                    )
                    .build()
                    .unwrap(),
            ),
        )]);

        let (filtered_txns, num_stripped) = strip_transactions(vec![txn.clone()], &filter);
        assert_eq!(num_stripped, 1);
        assert_eq!(filtered_txns.len(), 1);
        let txn = filtered_txns.first().unwrap();
        let user_transaction = match &txn.txn_data {
            Some(TxnData::User(user_transaction)) => user_transaction,
            _ => panic!("Expected user transaction"),
        };
        assert_eq!(user_transaction.request.as_ref().unwrap().payload, None);
        assert_eq!(user_transaction.request.as_ref().unwrap().signature, None);
        assert_eq!(user_transaction.events.len(), 0);
        assert_eq!(txn.info.as_ref().unwrap().changes.len(), 0);
    }

    #[test]
    fn test_transactions_are_stripped_correctly_module_name() {
        let txn = create_test_transaction(
            MODULE_ADDRESS.to_string(),
            MODULE_NAME.to_string(),
            FUNCTION_NAME.to_string(),
        );
        // Testing filter with only module set
        let filter = BooleanTransactionFilter::new_or(vec![BooleanTransactionFilter::from(
            APIFilter::UserTransactionFilter(
                UserTransactionFilterBuilder::default()
                    .payload(
                        UserTransactionPayloadFilterBuilder::default()
                            .function(
                                EntryFunctionFilterBuilder::default()
                                    .module(MODULE_NAME.to_string())
                                    .build()
                                    .unwrap(),
                            )
                            .build()
                            .unwrap(),
                    )
                    .build()
                    .unwrap(),
            ),
        )]);

        let (filtered_txns, num_stripped) = strip_transactions(vec![txn.clone()], &filter);
        assert_eq!(num_stripped, 1);
        assert_eq!(filtered_txns.len(), 1);
        let txn = filtered_txns.first().unwrap();
        let user_transaction = match &txn.txn_data {
            Some(TxnData::User(user_transaction)) => user_transaction,
            _ => panic!("Expected user transaction"),
        };
        assert_eq!(user_transaction.request.as_ref().unwrap().payload, None);
        assert_eq!(user_transaction.request.as_ref().unwrap().signature, None);
        assert_eq!(user_transaction.events.len(), 0);
        assert_eq!(txn.info.as_ref().unwrap().changes.len(), 0);
    }

    #[test]
    fn test_transactions_are_stripped_correctly_function_name() {
        let txn = create_test_transaction(
            MODULE_ADDRESS.to_string(),
            MODULE_NAME.to_string(),
            FUNCTION_NAME.to_string(),
        );
        // Testing filter with only function set
        let filter = BooleanTransactionFilter::new_or(vec![BooleanTransactionFilter::from(
            APIFilter::UserTransactionFilter(
                UserTransactionFilterBuilder::default()
                    .payload(
                        UserTransactionPayloadFilterBuilder::default()
                            .function(
                                EntryFunctionFilterBuilder::default()
                                    .function(FUNCTION_NAME.to_string())
                                    .build()
                                    .unwrap(),
                            )
                            .build()
                            .unwrap(),
                    )
                    .build()
                    .unwrap(),
            ),
        )]);

        let (filtered_txns, num_stripped) = strip_transactions(vec![txn.clone()], &filter);
        assert_eq!(num_stripped, 1);
        assert_eq!(filtered_txns.len(), 1);
        let txn = filtered_txns.first().unwrap();
        let user_transaction = match &txn.txn_data {
            Some(TxnData::User(user_transaction)) => user_transaction,
            _ => panic!("Expected user transaction"),
        };
        assert_eq!(user_transaction.request.as_ref().unwrap().payload, None);
        assert_eq!(user_transaction.request.as_ref().unwrap().signature, None);
        assert_eq!(user_transaction.events.len(), 0);
        assert_eq!(txn.info.as_ref().unwrap().changes.len(), 0);
    }
    #[test]
    fn test_transactions_are_not_stripped() {
        let txn = create_test_transaction(
            MODULE_ADDRESS.to_string(),
            MODULE_NAME.to_string(),
            FUNCTION_NAME.to_string(),
        );
        // Testing filter with wrong filter
        let filter = BooleanTransactionFilter::new_or(vec![BooleanTransactionFilter::from(
            APIFilter::UserTransactionFilter(
                UserTransactionFilterBuilder::default()
                    .payload(
                        UserTransactionPayloadFilterBuilder::default()
                            .function(
                                EntryFunctionFilterBuilder::default()
                                    .function("0xrandom".to_string())
                                    .build()
                                    .unwrap(),
                            )
                            .build()
                            .unwrap(),
                    )
                    .build()
                    .unwrap(),
            ),
        )]);

        let (filtered_txns, num_stripped) = strip_transactions(vec![txn.clone()], &filter);
        assert_eq!(num_stripped, 0);
        assert_eq!(filtered_txns.len(), 1);
        let txn = filtered_txns.first().unwrap();
        let user_transaction = match &txn.txn_data {
            Some(TxnData::User(user_transaction)) => user_transaction,
            _ => panic!("Expected user transaction"),
        };
        assert_ne!(user_transaction.request.as_ref().unwrap().payload, None);
        assert_ne!(user_transaction.request.as_ref().unwrap().signature, None);
        assert_ne!(user_transaction.events.len(), 0);
        assert_ne!(txn.info.as_ref().unwrap().changes.len(), 0);
    }
}
