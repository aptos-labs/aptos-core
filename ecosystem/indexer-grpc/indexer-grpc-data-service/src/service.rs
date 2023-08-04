// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{
    CONNECTION_COUNT, ERROR_COUNT, LATEST_PROCESSED_VERSION, PROCESSED_BATCH_SIZE,
    PROCESSED_LATENCY_IN_SECS, PROCESSED_LATENCY_IN_SECS_ALL, PROCESSED_VERSIONS_COUNT,
    SHORT_CONNECTION_COUNT,
};
use aptos_indexer_grpc_utils::{
    build_protobuf_encoded_transaction_wrappers,
    cache_operator::{CacheBatchGetStatus, CacheOperator},
    chunk_transactions,
    config::IndexerGrpcFileStoreConfig,
    constants::{
        BLOB_STORAGE_SIZE, GRPC_AUTH_TOKEN_HEADER, GRPC_REQUEST_NAME_HEADER, MESSAGE_SIZE_LIMIT,
    },
    file_store_operator::{FileStoreOperator, GcsFileStoreOperator, LocalFileStoreOperator},
    time_diff_since_pb_timestamp_in_secs, EncodedTransactionWithVersion,
};
use aptos_moving_average::MovingAverage;
use aptos_protos::{
    indexer::v1::{raw_data_server::RawData, GetTransactionsRequest, TransactionsResponse},
    transaction::v1::Transaction,
};
use futures::Stream;
use prost::Message;
use serde::{Deserialize, Serialize};
use std::{pin::Pin, sync::Arc, time::Duration};
use tokio::sync::mpsc::{channel, error::SendTimeoutError};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tracing::{error, info, warn, Instrument};
use uuid::Uuid;
type ResponseStream = Pin<Box<dyn Stream<Item = Result<TransactionsResponse, Status>> + Send>>;

#[derive(Clone, Serialize, Deserialize, Debug)]
struct RequestMetadata {
    pub request_id: String,
    pub request_remote_addr: String,
    pub request_token: String,
    pub request_name: String,
    pub request_source: String,
}

const MOVING_AVERAGE_WINDOW_SIZE: u64 = 10_000;
// When trying to fetch beyond the current head of cache, the server will retry after this duration.
const AHEAD_OF_CACHE_RETRY_SLEEP_DURATION_MS: u64 = 50;
// When error happens when fetching data from cache and file store, the server will retry after this duration.
// TODO(larry): fix all errors treated as transient errors.
const TRANSIENT_DATA_ERROR_RETRY_SLEEP_DURATION_MS: u64 = 1000;

// Up to MAX_RESPONSE_CHANNEL_SIZE response can be buffered in the channel. If the channel is full,
// the server will not fetch more data from the cache and file store until the channel is not full.
const MAX_RESPONSE_CHANNEL_SIZE: usize = 80;

// The server will retry to send the response to the client and give up after RESPONSE_CHANNEL_SEND_TIMEOUT.
// This is to prevent the server from being occupied by a slow client.
const RESPONSE_CHANNEL_SEND_TIMEOUT: Duration = Duration::from_secs(120);

const SHORT_CONNECTION_DURATION_IN_SECS: u64 = 10;

pub struct RawDataServerWrapper {
    pub redis_client: Arc<redis::Client>,
    pub file_store_config: IndexerGrpcFileStoreConfig,
}

impl RawDataServerWrapper {
    pub fn new(redis_address: String, file_store_config: IndexerGrpcFileStoreConfig) -> Self {
        Self {
            redis_client: Arc::new(
                redis::Client::open(format!("redis://{}", redis_address))
                    .expect("Create redis client failed."),
            ),
            file_store_config,
        }
    }
}

/// Enum to represent the status of the data fetching overall.
enum TransactionsDataStatus {
    // Data fetching is successful.
    Success(Vec<EncodedTransactionWithVersion>),
    // Ahead of current head of cache.
    AheadOfCache,
    // Fatal error when gap detected between cache and file store.
    DataGap,
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
        CONNECTION_COUNT.inc();
        let request = req.into_inner();

        let transactions_count = request.transactions_count;

        // Response channel to stream the data to the client.
        let (tx, rx) = channel(MAX_RESPONSE_CHANNEL_SIZE);
        let mut current_version = match &request.starting_version {
            Some(version) => *version,
            None => {
                return Result::Err(Status::aborted("Starting version is not set"));
            },
        };

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

        // Adds tracing context for the request.
        let serving_span = tracing::span!(
            tracing::Level::INFO,
            "Data Serving",
            request_id = request_metadata.request_id.as_str(),
            request_remote_addr = request_metadata.request_remote_addr.as_str(),
            request_token = request_metadata.request_token.as_str(),
            request_name = request_metadata.request_name.as_str(),
            request_source = request_metadata.request_source.as_str(),
        );

        let redis_client = self.redis_client.clone();
        tokio::spawn(
            async move {
                let mut connection_start_time = Some(std::time::Instant::now());
                let mut transactions_count = transactions_count;
                let conn = match redis_client.get_tokio_connection_manager().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        ERROR_COUNT
                            .with_label_values(&["redis_connection_failed"])
                            .inc();
                        SHORT_CONNECTION_COUNT.inc();
                        // Connection will be dropped anyway, so we ignore the error here.
                        let _result = tx
                            .send_timeout(
                                Err(Status::unavailable(
                                    "[Indexer Data] Cannot connect to Redis; please retry.",
                                )),
                                RESPONSE_CHANNEL_SEND_TIMEOUT,
                            )
                            .await;
                        error!(
                            error = e.to_string(),
                            "[Indexer Data] Failed to get redis connection."
                        );
                        return;
                    },
                };
                let mut cache_operator = CacheOperator::new(conn);
                file_store_operator.verify_storage_bucket_existence().await;

                let chain_id = match cache_operator.get_chain_id().await {
                    Ok(chain_id) => chain_id,
                    Err(e) => {
                        ERROR_COUNT
                            .with_label_values(&["redis_get_chain_id_failed"])
                            .inc();
                        SHORT_CONNECTION_COUNT.inc();
                        // Connection will be dropped anyway, so we ignore the error here.
                        let _result = tx
                            .send_timeout(
                                Err(Status::unavailable(
                                    "[Indexer Data] Cannot get the chain id; please retry.",
                                )),
                                RESPONSE_CHANNEL_SEND_TIMEOUT,
                            )
                            .await;
                        error!(
                            error = e.to_string(),
                            "[Indexer Data] Failed to get chain id."
                        );
                        return;
                    },
                };
                // Data service metrics.
                let mut tps_calculator = MovingAverage::new(MOVING_AVERAGE_WINDOW_SIZE);

                info!(
                    chain_id = chain_id,
                    current_version = current_version,
                    "[Indexer Data] New request received."
                );

                loop {
                    // 1. Fetch data from cache and file store.
                    let mut transaction_data = match data_fetch(
                        current_version,
                        &mut cache_operator,
                        file_store_operator.as_ref(),
                    )
                    .await
                    {
                        Ok(TransactionsDataStatus::Success(transactions)) => transactions,
                        Ok(TransactionsDataStatus::AheadOfCache) => {
                            ahead_of_cache_data_handling().await;
                            // Retry after a short sleep.
                            continue;
                        },
                        Ok(TransactionsDataStatus::DataGap) => {
                            data_gap_handling(current_version);
                            // End the data stream.
                            break;
                        },
                        Err(e) => {
                            ERROR_COUNT.with_label_values(&["data_fetch_failed"]).inc();
                            data_fetch_error_handling(e, current_version, chain_id).await;
                            // Retry after a short sleep.
                            continue;
                        },
                    };
                    if let Some(count) = transactions_count {
                        if count == 0 {
                            // End the data stream.
                            // Since the client receives all the data it requested, we don't count it as a short conneciton.
                            connection_start_time = None;
                            break;
                        } else if (count as usize) < transaction_data.len() {
                            // Trim the data to the requested size.
                            transaction_data.truncate(count as usize);
                            transactions_count = Some(0);
                        } else {
                            transactions_count = Some(count - transaction_data.len() as u64);
                        }
                    };
                    // 2. Push the data to the response channel, i.e. stream the data to the client.
                    let current_batch_size = transaction_data.as_slice().len();
                    let end_of_batch_version = transaction_data.as_slice().last().unwrap().1;
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

                    match channel_send_multiple_with_timeout(resp_items, tx.clone()).await {
                        Ok(_) => {
                            PROCESSED_BATCH_SIZE
                                .with_label_values(&[
                                    request_metadata.request_token.as_str(),
                                    request_metadata.request_name.as_str(),
                                ])
                                .set(current_batch_size as i64);
                            LATEST_PROCESSED_VERSION
                                .with_label_values(&[
                                    request_metadata.request_token.as_str(),
                                    request_metadata.request_name.as_str(),
                                ])
                                .set(end_of_batch_version as i64);
                            PROCESSED_VERSIONS_COUNT
                                .with_label_values(&[
                                    request_metadata.request_token.as_str(),
                                    request_metadata.request_name.as_str(),
                                ])
                                .inc_by(current_batch_size as u64);
                            if let Some(data_latency_in_secs) = data_latency_in_secs {
                                // If it's a partial batch, we record the latency because it usually means
                                // the data is the latest.
                                if current_batch_size % BLOB_STORAGE_SIZE != 0 {
                                    PROCESSED_LATENCY_IN_SECS
                                        .with_label_values(&[
                                            request_metadata.request_token.as_str(),
                                            request_metadata.request_name.as_str(),
                                        ])
                                        .set(data_latency_in_secs);
                                    PROCESSED_LATENCY_IN_SECS_ALL
                                        .with_label_values(&[request_metadata
                                            .request_source
                                            .as_str()])
                                        .observe(data_latency_in_secs);
                                }
                            }
                        },
                        Err(SendTimeoutError::Timeout(_)) => {
                            warn!("[Indexer Data] Receiver is full; exiting.");
                            break;
                        },
                        Err(SendTimeoutError::Closed(_)) => {
                            warn!("[Indexer Data] Receiver is closed; exiting.");
                            break;
                        },
                    }
                    // 3. Update the current version and record current tps.
                    tps_calculator.tick_now(current_batch_size as u64);
                    current_version = end_of_batch_version + 1;
                    info!(
                        current_version = current_version,
                        end_version = end_of_batch_version,
                        batch_size = current_batch_size,
                        tps = (tps_calculator.avg() * 1000.0) as u64,
                        "[Indexer Data] Sending batch."
                    );
                }
                info!("[Indexer Data] Client disconnected.");
                if let Some(start_time) = connection_start_time {
                    if start_time.elapsed().as_secs() < SHORT_CONNECTION_DURATION_IN_SECS {
                        SHORT_CONNECTION_COUNT.inc();
                    }
                }
            }
            .instrument(serving_span),
        );

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::GetTransactionsStream
        ))
    }
}

/// Builds the response for the get transactions request. Partial batch is ok, i.e., a batch with transactions < 1000.
fn get_transactions_responses_builder(
    data: Vec<EncodedTransactionWithVersion>,
    chain_id: u32,
) -> Vec<TransactionsResponse> {
    let transactions: Vec<Transaction> = data
        .into_iter()
        .map(|(encoded, _)| {
            let decoded_transaction = base64::decode(encoded).unwrap();
            let transaction = Transaction::decode(&*decoded_transaction);
            transaction.unwrap()
        })
        .collect();
    let chunks = chunk_transactions(transactions, MESSAGE_SIZE_LIMIT);
    chunks
        .into_iter()
        .map(|chunk| TransactionsResponse {
            chain_id: Some(chain_id as u64),
            transactions: chunk,
        })
        .collect::<Vec<TransactionsResponse>>()
}

/// Fetches data from cache or the file store. It returns the data if it is ready in the cache or file store.
/// Otherwise, it returns the status of the data fetching.
async fn data_fetch(
    starting_version: u64,
    cache_operator: &mut CacheOperator<redis::aio::ConnectionManager>,
    file_store_operator: &dyn FileStoreOperator,
) -> anyhow::Result<TransactionsDataStatus> {
    let batch_get_result = cache_operator
        .batch_get_encoded_proto_data(starting_version)
        .await;

    match batch_get_result {
        // Data is not ready yet in the cache.
        Ok(CacheBatchGetStatus::NotReady) => Ok(TransactionsDataStatus::AheadOfCache),
        Ok(CacheBatchGetStatus::Ok(transactions)) => Ok(TransactionsDataStatus::Success(
            build_protobuf_encoded_transaction_wrappers(transactions, starting_version),
        )),
        Ok(CacheBatchGetStatus::EvictedFromCache) => {
            // Data is evicted from the cache. Fetch from file store.
            let file_store_batch_get_result =
                file_store_operator.get_transactions(starting_version).await;
            match file_store_batch_get_result {
                Ok(transactions) => Ok(TransactionsDataStatus::Success(
                    build_protobuf_encoded_transaction_wrappers(transactions, starting_version),
                )),
                Err(e) => {
                    if e.to_string().contains("Transactions file not found") {
                        Ok(TransactionsDataStatus::DataGap)
                    } else {
                        Err(e)
                    }
                },
            }
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

/// Handles data gap errors, i.e., the data is not present in the cache or file store.
fn data_gap_handling(version: u64) {
    // TODO(larry): add metrics/alerts to track the gap.
    // Do not crash the server when gap detected since other clients may still be able to get data.
    error!(
        current_version = version,
        "[Indexer Data] Data gap detected. Please check the logs for more details."
    );
}

/// Handles data fetch errors, including cache and file store related errors.
async fn data_fetch_error_handling(err: anyhow::Error, current_version: u64, chain_id: u64) {
    error!(
        chain_id = chain_id,
        current_version = current_version,
        "[Indexer Data] Failed to fetch data from cache and file store. {:?}",
        err
    );
    tokio::time::sleep(Duration::from_millis(
        TRANSIENT_DATA_ERROR_RETRY_SLEEP_DURATION_MS,
    ))
    .await;
}

/// Gets the request metadata. Useful for logging.
fn get_request_metadata(req: &Request<GetTransactionsRequest>) -> tonic::Result<RequestMetadata> {
    // Request id.
    let request_id = Uuid::new_v4().to_string();

    let request_token = match req
        .metadata()
        .get(GRPC_AUTH_TOKEN_HEADER)
        .map(|token| token.to_str())
    {
        Some(Ok(token)) => token.to_string(),
        // It's required to have a valid request token.
        _ => return Result::Err(Status::aborted("Invalid request token")),
    };

    let request_remote_addr = match req.remote_addr() {
        Some(addr) => addr.to_string(),
        None => return Result::Err(Status::aborted("Invalid remote address")),
    };
    let request_name = match req
        .metadata()
        .get(GRPC_REQUEST_NAME_HEADER)
        .map(|desc| desc.to_str())
    {
        Some(Ok(desc)) => desc.to_string(),
        // If the request description is not provided, use "unknown".
        _ => "unknown".to_string(),
    };
    Ok(RequestMetadata {
        request_id,
        request_remote_addr,
        request_token,
        request_name,
        // TODO: after launch, support 'core', 'partner', 'community' and remove 'testing_v1'.
        request_source: "testing_v1".to_string(),
    })
}

async fn channel_send_multiple_with_timeout(
    resp_items: Vec<TransactionsResponse>,
    tx: tokio::sync::mpsc::Sender<Result<TransactionsResponse, Status>>,
) -> Result<(), SendTimeoutError<Result<TransactionsResponse, Status>>> {
    for resp_item in resp_items {
        let current_instant = std::time::Instant::now();
        tx.send_timeout(
            Result::<TransactionsResponse, Status>::Ok(resp_item),
            RESPONSE_CHANNEL_SEND_TIMEOUT,
        )
        .await?;
        info!(
            "[data service] response waiting time in seconds: {}",
            current_instant.elapsed().as_secs_f64()
        );
    }
    Ok(())
}
