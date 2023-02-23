// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_utils::{
    cache_operator::{CacheBatchGetStatus, CacheOperator},
    config::IndexerGrpcConfig,
    constants::BLOB_STORAGE_SIZE,
    file_store_operator::FileStoreOperator,
};
use aptos_logger::{info, warn};
use aptos_moving_average::MovingAverage;
use aptos_protos::datastream::v1::{
    indexer_stream_server::IndexerStream,
    raw_datastream_response::Response as DatastreamProtoResponse, RawDatastreamRequest,
    RawDatastreamResponse, TransactionOutput, TransactionsOutput,
};
use futures::Stream;
use std::{pin::Pin, sync::Arc, thread::sleep, time::Duration};
use tokio::sync::mpsc::{channel, error::TrySendError};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use uuid::Uuid;

type ResponseStream = Pin<Box<dyn Stream<Item = Result<RawDatastreamResponse, Status>> + Send>>;

const MOVING_AVERAGE_WINDOW_SIZE: u64 = 10_000;
const DATA_NOT_READY_SLEEP_DURATION_MS: u64 = 1000;

pub struct DatastreamServer {
    pub redis_client: Arc<redis::Client>,
    pub config: IndexerGrpcConfig,
}

impl DatastreamServer {
    pub fn new(config: IndexerGrpcConfig) -> Self {
        Self {
            redis_client: Arc::new(
                redis::Client::open(format!("redis://{}", config.redis_address))
                    .expect("Create redis client failed."),
            ),
            config,
        }
    }
}

// The hard limit of TPS to avoid overloading the server.
const MAX_TPS: u64 = 20_000;
// The backoff time when the channel is full, in other words, stop fetching data from the storage.
const CHANNEL_FULL_BACKOFF_IN_SECS: u64 = 1;
const STREAMING_CHANNEL_SIZE: u64 =
    MAX_TPS * CHANNEL_FULL_BACKOFF_IN_SECS / BLOB_STORAGE_SIZE as u64;

// DatastreamServer handles the raw datastream requests from cache and file store.
#[tonic::async_trait]
impl IndexerStream for DatastreamServer {
    type RawDatastreamStream = ResponseStream;

    async fn raw_datastream(
        &self,
        req: Request<RawDatastreamRequest>,
    ) -> Result<Response<Self::RawDatastreamStream>, Status> {
        // Limit the TPS at 20K. This is to prevent the server from being overloaded.
        let (tx, rx) = channel(STREAMING_CHANNEL_SIZE as usize);
        let req = req.into_inner();
        // Round the version to the nearest BLOB_STORAGE_SIZE.
        let mut current_version =
            (req.starting_version / BLOB_STORAGE_SIZE as u64) * BLOB_STORAGE_SIZE as u64;

        let file_store_bucket_name = self.config.file_store_bucket_name.clone();
        let redis_client = self.redis_client.clone();
        tokio::spawn(async move {
            let mut ma = MovingAverage::new(MOVING_AVERAGE_WINDOW_SIZE);
            let request_id = Uuid::new_v4().to_string();
            let conn = redis_client.get_async_connection().await.unwrap();
            let mut cache_operator = CacheOperator::new(conn);
            let chain_id = cache_operator.get_chain_id().await.unwrap();

            let file_store_operator = FileStoreOperator::new(file_store_bucket_name);
            file_store_operator.bootstrap().await;
            loop {
                // Check if the receiver is closed.
                if tx.is_closed() {
                    break;
                }

                let batch_get_result = cache_operator
                    .batch_get_encoded_proto_data(current_version)
                    .await;
                let encoded_proto_data_vec = match batch_get_result {
                    Ok(CacheBatchGetStatus::NotReady) => {
                        // Data is not ready yet in the cache.
                        sleep(Duration::from_millis(DATA_NOT_READY_SLEEP_DURATION_MS));
                        continue;
                    },
                    Ok(CacheBatchGetStatus::Ok(v)) => v,
                    Ok(CacheBatchGetStatus::HitTheHead(v)) => v,
                    Ok(CacheBatchGetStatus::EvictedFromCache) => {
                        // TODO: fetch from the file store.
                        continue;
                    },
                    Err(e) => {
                        warn!(
                            "[Indexer Data] Failed to get cache transactions. Error: {:?}",
                            e
                        );
                        sleep(Duration::from_millis(100));
                        continue;
                    },
                };
                let current_batch_size = encoded_proto_data_vec.len() as u64;
                let item = RawDatastreamResponse {
                    response: Some(DatastreamProtoResponse::Data(TransactionsOutput {
                        transactions: encoded_proto_data_vec
                            .iter()
                            .enumerate()
                            .map(|(i, e)| TransactionOutput {
                                encoded_proto_data: e.clone(),
                                version: current_version + i as u64,
                                ..TransactionOutput::default()
                            })
                            .collect(),
                    })),
                    chain_id: chain_id as u32,
                };
                match tx.try_send(Result::<_, Status>::Ok(item.clone())) {
                    Ok(_) => {},
                    Err(TrySendError::Full(_)) => {
                        warn!(
                            request_id = request_id.as_str(),
                            "[Indexer Data] Receiver is full; retrying."
                        );
                        std::thread::sleep(Duration::from_secs(1));
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
                current_version += current_batch_size;
                ma.tick_now(current_batch_size);
                info!(
                    request_id = request_id.as_str(),
                    current_version = current_version,
                    batch_size = current_batch_size,
                    tps = (ma.avg() * 1000.0) as u64,
                    "[Indexer Data] Sending batch."
                );
            }
            info!("[Indexer Data] Client disconnected.");
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::RawDatastreamStream
        ))
    }
}
