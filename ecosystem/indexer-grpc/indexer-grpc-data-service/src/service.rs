// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{
    BYTES_READY_TO_TRANSFER_FROM_SERVER, CONNECTION_COUNT, ERROR_COUNT,
    LATEST_PROCESSED_VERSION as LATEST_PROCESSED_VERSION_OLD, PROCESSED_BATCH_SIZE,
    PROCESSED_LATENCY_IN_SECS, PROCESSED_LATENCY_IN_SECS_ALL, PROCESSED_VERSIONS_COUNT,
    SHORT_CONNECTION_COUNT,
};
use anyhow::{Context, Result};
use aptos_indexer_grpc_utils::{
    cache_operator::{CacheBatchGetStatus, CacheOperator},
    chunk_transactions,
    compression_util::{CacheEntry, StorageFormat},
    config::IndexerGrpcFileStoreConfig,
    constants::{
        IndexerGrpcRequestMetadata, GRPC_AUTH_TOKEN_HEADER, GRPC_REQUEST_NAME_HEADER,
        MESSAGE_SIZE_LIMIT,
    },
    counters::{log_grpc_step, IndexerGrpcStep},
    file_store_operator::FileStoreOperator,
    time_diff_since_pb_timestamp_in_secs,
    types::RedisUrl,
};
use aptos_moving_average::MovingAverage;
use aptos_protos::{
    indexer::v1::{raw_data_server::RawData, GetTransactionsRequest, TransactionsResponse},
    transaction::v1::Transaction,
};
use futures::Stream;
use prost::Message;
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

const REQUEST_HEADER_APTOS_EMAIL_HEADER: &str = "x-aptos-email";
const REQUEST_HEADER_APTOS_USER_CLASSIFICATION_HEADER: &str = "x-aptos-user-classification";
const REQUEST_HEADER_APTOS_API_KEY_NAME: &str = "x-aptos-api-key-name";
const RESPONSE_HEADER_APTOS_CONNECTION_ID_HEADER: &str = "x-aptos-connection-id";
const SERVICE_TYPE: &str = "data_service";

pub struct RawDataServerWrapper {
    pub redis_client: Arc<redis::Client>,
    pub file_store_config: IndexerGrpcFileStoreConfig,
    pub data_service_response_channel_size: usize,
    pub cache_storage_format: StorageFormat,
}

impl RawDataServerWrapper {
    pub fn new(
        redis_address: RedisUrl,
        file_store_config: IndexerGrpcFileStoreConfig,
        data_service_response_channel_size: usize,
        cache_storage_format: StorageFormat,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            redis_client: Arc::new(
                redis::Client::open(redis_address.0.clone()).with_context(|| {
                    format!("Failed to create redis client for {}", redis_address)
                })?,
            ),
            file_store_config,
            data_service_response_channel_size,
            cache_storage_format,
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
            .with_label_values(&[
                request_metadata.request_api_key_name.as_str(),
                request_metadata.request_email.as_str(),
                request_metadata.processor_name.as_str(),
            ])
            .inc();
        let request = req.into_inner();

        let transactions_count = request.transactions_count;

        // Response channel to stream the data to the client.
        let (tx, rx) = channel(self.data_service_response_channel_size);
        let mut current_version = match &request.starting_version {
            Some(version) => *version,
            None => {
                return Result::Err(Status::aborted("Starting version is not set"));
            },
        };

        let file_store_operator: Box<dyn FileStoreOperator> = self.file_store_config.create();

        // Adds tracing context for the request.
        log_grpc_step(
            SERVICE_TYPE,
            IndexerGrpcStep::DataServiceNewRequestReceived,
            Some(current_version as i64),
            None,
            None,
            None,
            None,
            None,
            None,
            Some(request_metadata.clone()),
        );

        let redis_client = self.redis_client.clone();
        let cache_storage_format = self.cache_storage_format;
        tokio::spawn({
            let request_metadata = request_metadata.clone();
            async move {
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
                    let current_batch_start_time = std::time::Instant::now();
                    let mut transaction_data = match data_fetch(
                        current_version,
                        &mut cache_operator,
                        file_store_operator.as_ref(),
                        request_metadata.clone(),
                        cache_storage_format,
                    )
                    .await
                    {
                        Ok(TransactionsDataStatus::Success(transactions)) => transactions,
                        Ok(TransactionsDataStatus::AheadOfCache) => {
                            info!(
                                start_version = current_version,
                                request_name = request_metadata.processor_name.as_str(),
                                request_email = request_metadata.request_email.as_str(),
                                request_api_key_name = request_metadata.request_api_key_name.as_str(),
                                processor_name = request_metadata.processor_name.as_str(),
                                connection_id = request_metadata.request_connection_id.as_str(),
                    request_user_classification =
                        request_metadata.request_user_classification.as_str(),
                                duration_in_secs = current_batch_start_time.elapsed().as_secs_f64(),
                                service_type = SERVICE_TYPE,
                                "[Data Service] Requested data is ahead of cache. Sleeping for {} ms.",
                                AHEAD_OF_CACHE_RETRY_SLEEP_DURATION_MS,
                            );
                            ahead_of_cache_data_handling().await;
                            // Retry after a short sleep.
                            continue;
                        },
                        Err(e) => {
                            ERROR_COUNT.with_label_values(&["data_fetch_failed"]).inc();
                            data_fetch_error_handling(e, current_version, chain_id).await;
                            // Retry after a short sleep.
                            continue;
                        },
                    };

                    // TODO: Unify the truncation logic for start and end.
                    if let Some(count) = transactions_count {
                        if count == 0 {
                            // End the data stream.
                            // Since the client receives all the data it requested, we don't count it as a short conneciton.
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
                        .with_label_values(&[
                            request_metadata.request_api_key_name.as_str(),
                            request_metadata.request_email.as_str(),
                            request_metadata.processor_name.as_str(),
                        ])
                        .inc_by(bytes_ready_to_transfer as u64);
                    // 2. Push the data to the response channel, i.e. stream the data to the client.
                    let current_batch_size = transaction_data.as_slice().len();
                    let end_of_batch_version = transaction_data.as_slice().last().unwrap().version;
                    let resp_items =
                        get_transactions_responses_builder(transaction_data, chain_id as u32);
                    let data_latency_in_secs = resp_items
                        .last()
                        .unwrap()
                        .transactions
                        .last()
                        .unwrap()
                        .timestamp
                        .as_ref()
                        .map(time_diff_since_pb_timestamp_in_secs);

                    match channel_send_multiple_with_timeout(
                        resp_items,
                        tx.clone(),
                        request_metadata.clone(),
                    )
                    .await
                    {
                        Ok(_) => {
                            PROCESSED_BATCH_SIZE
                                .with_label_values(&[
                                    request_metadata.request_api_key_name.as_str(),
                                    request_metadata.request_email.as_str(),
                                    request_metadata.processor_name.as_str(),
                                ])
                                .set(current_batch_size as i64);
                            // TODO: Reasses whether this metric useful
                            LATEST_PROCESSED_VERSION_OLD
                                .with_label_values(&[
                                    request_metadata.request_api_key_name.as_str(),
                                    request_metadata.request_email.as_str(),
                                    request_metadata.processor_name.as_str(),
                                ])
                                .set(end_of_batch_version as i64);
                            PROCESSED_VERSIONS_COUNT
                                .with_label_values(&[
                                    request_metadata.request_api_key_name.as_str(),
                                    request_metadata.request_email.as_str(),
                                    request_metadata.processor_name.as_str(),
                                ])
                                .inc_by(current_batch_size as u64);
                            if let Some(data_latency_in_secs) = data_latency_in_secs {
                                PROCESSED_LATENCY_IN_SECS
                                    .with_label_values(&[
                                        request_metadata.request_api_key_name.as_str(),
                                        request_metadata.request_email.as_str(),
                                        request_metadata.processor_name.as_str(),
                                    ])
                                    .set(data_latency_in_secs);
                                PROCESSED_LATENCY_IN_SECS_ALL
                                    .with_label_values(&[request_metadata
                                        .request_user_classification
                                        .as_str()])
                                    .observe(data_latency_in_secs);
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
                    request_name = request_metadata.processor_name.as_str(),
                    request_email = request_metadata.request_email.as_str(),
                    request_api_key_name = request_metadata.request_api_key_name.as_str(),
                    processor_name = request_metadata.processor_name.as_str(),
                    connection_id = request_metadata.request_connection_id.as_str(),
                    request_user_classification =
                        request_metadata.request_user_classification.as_str(),
                    request_user_classification =
                        request_metadata.request_user_classification.as_str(),
                    service_type = SERVICE_TYPE,
                    "[Data Service] Client disconnected."
                );
                if let Some(start_time) = connection_start_time {
                    if start_time.elapsed().as_secs() < SHORT_CONNECTION_DURATION_IN_SECS {
                        SHORT_CONNECTION_COUNT
                            .with_label_values(&[
                                request_metadata.request_api_key_name.as_str(),
                                request_metadata.request_email.as_str(),
                                request_metadata.processor_name.as_str(),
                            ])
                            .inc();
                    }
                }
            }
        });

        let output_stream = ReceiverStream::new(rx);
        let mut response = Response::new(Box::pin(output_stream) as Self::GetTransactionsStream);

        response.metadata_mut().insert(
            RESPONSE_HEADER_APTOS_CONNECTION_ID_HEADER,
            tonic::metadata::MetadataValue::from_str(
                request_metadata.request_connection_id.as_str(),
            )
            .unwrap(),
        );
        Ok(response)
    }
}

/// Builds the response for the get transactions request. Partial batch is ok, i.e., a batch with transactions < 1000.
fn get_transactions_responses_builder(
    transactions: Vec<Transaction>,
    chain_id: u32,
) -> Vec<TransactionsResponse> {
    let chunks = chunk_transactions(transactions, MESSAGE_SIZE_LIMIT);
    chunks
        .into_iter()
        .map(|chunk| TransactionsResponse {
            chain_id: Some(chain_id as u64),
            transactions: chunk,
        })
        .collect()
}

/// Fetches data from cache or the file store. It returns the data if it is ready in the cache or file store.
/// Otherwise, it returns the status of the data fetching.
async fn data_fetch(
    starting_version: u64,
    cache_operator: &mut CacheOperator<redis::aio::ConnectionManager>,
    file_store_operator: &dyn FileStoreOperator,
    request_metadata: IndexerGrpcRequestMetadata,
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
            let transactions = transactions
                .into_iter()
                .map(|transaction| {
                    let cache_entry = CacheEntry::new(transaction, storage_format);
                    cache_entry.into_transaction()
                })
                .collect::<Vec<Transaction>>();
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
                Some(request_metadata.clone()),
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
                Some(request_metadata.clone()),
            );

            Ok(TransactionsDataStatus::Success(transactions))
        },
        Ok(CacheBatchGetStatus::EvictedFromCache) => {
            // Data is evicted from the cache. Fetch from file store.
            let (transactions, io_duration, decoding_duration) = file_store_operator
                .get_transactions_with_durations(starting_version)
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
                Some(request_metadata.clone()),
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
                Some(request_metadata.clone()),
            );
            Ok(TransactionsDataStatus::Success(transactions))
        },
        Err(e) => Err(e),
    }
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
        ("request_api_key_name", REQUEST_HEADER_APTOS_API_KEY_NAME),
        ("request_email", REQUEST_HEADER_APTOS_EMAIL_HEADER),
        (
            "request_user_classification",
            REQUEST_HEADER_APTOS_USER_CLASSIFICATION_HEADER,
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
    request_metadata: IndexerGrpcRequestMetadata,
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
            Some(request_metadata.clone()),
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
        Some(request_metadata.clone()),
    );

    Ok(())
}
