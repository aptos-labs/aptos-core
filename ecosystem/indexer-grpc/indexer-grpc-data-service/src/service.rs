// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_utils::{
    build_protobuf_encoded_transaction_wrappers,
    cache_operator::{CacheBatchGetStatus, CacheOperator},
    config::IndexerGrpcConfig,
    file_store_operator::FileStoreOperator,
    EncodedTransactionWithVersion,
};
use aptos_logger::{error, info, warn};
use aptos_moving_average::MovingAverage;
use aptos_protos::datastream::v1::{
    indexer_stream_server::IndexerStream,
    raw_datastream_response::Response as DatastreamProtoResponse, RawDatastreamRequest,
    RawDatastreamResponse, StreamStatus, TransactionOutput, TransactionsOutput,
};
use futures::Stream;
use std::{pin::Pin, sync::Arc, time::Duration};
use tokio::sync::mpsc::{channel, error::TrySendError};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use uuid::Uuid;

type ResponseStream = Pin<Box<dyn Stream<Item = Result<RawDatastreamResponse, Status>> + Send>>;

const MOVING_AVERAGE_WINDOW_SIZE: u64 = 10_000;
// When trying to fetch beyond the current head of cache, the server will retry after this duration.
const AHEAD_OF_CACHE_RETRY_SLEEP_DURATION_MS: u64 = 200;
// When error happens when fetching data from cache and file store, the server will retry after this duration.
// TODO(larry): fix all errors treated as transient errors.
const TRANSIENT_DATA_ERROR_RETRY_SLEEP_DURATION_MS: u64 = 1000;

// TODO(larry): replace this with a exponential backoff.
// The server will not fetch more data from the cache and file store until the channel is not full.
const RESPONSE_CHANNEL_FULL_BACKOFF_DURATION_MS: u64 = 1000;
// Up to MAX_RESPONSE_CHANNEL_SIZE response can be buffered in the channel. If the channel is full,
// the server will not fetch more data from the cache and file store until the channel is not full.
const MAX_RESPONSE_CHANNEL_SIZE: usize = 40;

pub struct DatastreamServer {
    pub redis_client: Arc<redis::Client>,
    pub server_config: IndexerGrpcConfig,
}

impl DatastreamServer {
    pub fn new(config: IndexerGrpcConfig) -> Self {
        Self {
            redis_client: Arc::new(
                redis::Client::open(format!("redis://{}", config.redis_address))
                    .expect("Create redis client failed."),
            ),
            server_config: config,
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

/// DatastreamServer handles the raw datastream requests from cache and file store.
#[tonic::async_trait]
impl IndexerStream for DatastreamServer {
    type RawDatastreamStream = ResponseStream;

    /// RawDatastream is a streaming GRPC endpoint:
    /// 1. Fetches data from cache and file store.
    ///    1.1. If the data is beyond the current head of cache, retry after a short sleep.
    ///    1.2. If the data is not in cache, fetch the data from file store.
    ///    1.3. If the data is not in file store, stream connection will break.
    ///    1.4  If error happens, retry after a short sleep.
    /// 2. Push data into channel to stream to the client.
    ///    2.1. If the channel is full, do not fetch and retry after a short sleep.
    async fn raw_datastream(
        &self,
        req: Request<RawDatastreamRequest>,
    ) -> Result<Response<Self::RawDatastreamStream>, Status> {
        // Response channel to stream the data to the client.
        let (tx, rx) = channel(MAX_RESPONSE_CHANNEL_SIZE);
        let req = req.into_inner();
        let mut current_version = req.starting_version;

        let file_store_bucket_name = self.server_config.file_store_bucket_name.clone();
        let redis_client = self.redis_client.clone();

        tokio::spawn(async move {
            let conn = redis_client.get_async_connection().await.unwrap();
            let mut cache_operator = CacheOperator::new(conn);
            let file_store_operator = FileStoreOperator::new(file_store_bucket_name);
            file_store_operator.verify_storage_bucket_existence().await;

            let chain_id = cache_operator.get_chain_id().await.unwrap();
            // Data service metrics.
            let mut tps_calculator = MovingAverage::new(MOVING_AVERAGE_WINDOW_SIZE);
            // Request metadata.
            let request_id = Uuid::new_v4().to_string();
            info!(
                chain_id = chain_id,
                request_id = request_id.as_str(),
                current_version = current_version,
                "[Indexer Data] New request received."
            );
            tx.send(Ok(RawDatastreamResponse {
                chain_id: chain_id as u32,
                response: Some(DatastreamProtoResponse::Status(StreamStatus {
                    r#type: 1,
                    start_version: current_version,
                    ..StreamStatus::default()
                })),
            }))
            .await
            .unwrap();
            loop {
                // 1. Fetch data from cache and file store.
                let transaction_data =
                    match data_fetch(current_version, &mut cache_operator, &file_store_operator)
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
                            data_fetch_error_handling(
                                e,
                                current_version,
                                chain_id,
                                request_id.as_str(),
                            )
                            .await;
                            // Retry after a short sleep.
                            continue;
                        },
                    };

                // 2. Push the data to the response channel, i.e. stream the data to the client.
                let current_batch_size = transaction_data.len();
                let end_of_batch_version = transaction_data.last().unwrap().1;
                let resp_item = raw_datastream_response_builder(transaction_data, chain_id as u32);
                match tx.try_send(Result::<RawDatastreamResponse, Status>::Ok(resp_item)) {
                    Ok(_) => {},
                    Err(TrySendError::Full(_)) => {
                        warn!(
                            request_id = request_id.as_str(),
                            "[Indexer Data] Receiver is full; retrying."
                        );
                        tokio::time::sleep(Duration::from_millis(
                            RESPONSE_CHANNEL_FULL_BACKOFF_DURATION_MS,
                        ))
                        .await;
                        continue;
                    },
                    Err(TrySendError::Closed(_)) => {
                        warn!(
                            request_id = request_id.as_str(),
                            "[Indexer Data] Receiver is closed; exiting."
                        );
                        break;
                    },
                }
                // 3. Update the current version and record current tps.
                tps_calculator.tick_now(current_batch_size as u64);
                current_version = end_of_batch_version + 1;
                info!(
                    request_id = request_id.as_str(),
                    current_version = current_version,
                    batch_size = current_batch_size,
                    tps = (tps_calculator.avg() * 1000.0) as u64,
                    "[Indexer Data] Sending batch."
                );
            }
            info!(
                request_id = request_id.as_str(),
                "[Indexer Data] Client disconnected."
            );
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::RawDatastreamStream
        ))
    }
}

/// Builds the response for the raw datastream request. Partial batch is ok, i.e., a batch with transactions < 1000.
fn raw_datastream_response_builder(
    data: Vec<EncodedTransactionWithVersion>,
    chain_id: u32,
) -> RawDatastreamResponse {
    RawDatastreamResponse {
        response: Some(DatastreamProtoResponse::Data(TransactionsOutput {
            transactions: data
                .into_iter()
                .map(|(encoded, version)| TransactionOutput {
                    encoded_proto_data: encoded,
                    version,
                    ..TransactionOutput::default()
                })
                .collect(),
        })),
        chain_id,
    }
}

/// Fetches data from cache or the file store. It returns the data if it is ready in the cache or file store.
/// Otherwise, it returns the status of the data fetching.
async fn data_fetch(
    starting_version: u64,
    cache_operator: &mut CacheOperator<redis::aio::Connection>,
    file_store_operator: &FileStoreOperator,
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
async fn data_fetch_error_handling(
    err: anyhow::Error,
    current_version: u64,
    chain_id: u64,
    request_id: &str,
) {
    error!(
        request_id = request_id,
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
