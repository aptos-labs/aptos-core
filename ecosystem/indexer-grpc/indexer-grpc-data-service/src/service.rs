// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_utils::{
    get_cache_coverage_status, get_cache_transactions, get_file_store_bucket_name,
    get_redis_address,
    storage::{generate_blob_name, TransactionsBlob, BLOB_TRANSACTION_CHUNK_SIZE},
    CacheCoverageStatus,
};
use aptos_logger::{info, warn};
use aptos_moving_average::MovingAverage;
use aptos_protos::datastream::v1::{
    indexer_stream_server::IndexerStream,
    raw_datastream_response::Response as DatastreamProtoResponse, RawDatastreamRequest,
    RawDatastreamResponse, TransactionOutput, TransactionsOutput,
};
use cloud_storage::Object;
use futures::Stream;
use redis::{Client, Commands};
use std::{pin::Pin, thread::sleep, time::Duration};
use tokio::sync::mpsc::{self, error::TrySendError};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use uuid::Uuid;

type ResponseStream = Pin<Box<dyn Stream<Item = Result<RawDatastreamResponse, Status>> + Send>>;

const MOVING_AVERAGE_WINDOW_SIZE: u64 = 10_000;
const DATA_NOT_READY_SLEEP_DURATION: u64 = 1000;

pub struct DatastreamServer {
    pub redis_client: Client,
}

impl DatastreamServer {
    pub fn new() -> Self {
        Self {
            redis_client: Client::open(format!("redis://{}", get_redis_address())).unwrap(),
        }
    }
}

impl Default for DatastreamServer {
    fn default() -> Self {
        Self::new()
    }
}

// DatastreamServer handles the raw datastream requests from cache and file store.
#[tonic::async_trait]
impl IndexerStream for DatastreamServer {
    type RawDatastreamStream = ResponseStream;

    async fn raw_datastream(
        &self,
        req: Request<RawDatastreamRequest>,
    ) -> Result<Response<Self::RawDatastreamStream>, Status> {
        let (tx, rx) = mpsc::channel(10000);

        let mut conn = self.redis_client.get_connection().unwrap();

        let req = req.into_inner();
        // Round the version to the nearest STORAGE_BLOB_SIZE.
        let mut current_version =
            (req.starting_version / BLOB_TRANSACTION_CHUNK_SIZE) * BLOB_TRANSACTION_CHUNK_SIZE;

        tokio::spawn(async move {
            let mut ma = MovingAverage::new(MOVING_AVERAGE_WINDOW_SIZE);
            let request_id = Uuid::new_v4().to_string();
            let chain_id = conn
                .get("chain_id")
                .expect("[Indexer Data] Failed to get chain id.");
            let bucket_name = get_file_store_bucket_name();
            loop {
                // Check if the receiver is closed.
                if tx.is_closed() {
                    break;
                }

                // Get the cache coverage status.
                //  1. If the cache coverage status is CacheHit, it'll fetch the data from the cache.
                //  2. If the cache coverage status is CacheEvicted, it'll fetch the data from the file store.
                //  3. If the cache coverage status is DataNotReady, it'll wait and retry.
                let cache_coverage_status = get_cache_coverage_status(&mut conn, current_version)
                    .await
                    .expect("[Indexer Data] Failed to get cache coverage status.");

                let encoded_proto_data_vec = match cache_coverage_status {
                    CacheCoverageStatus::CacheEvicted => {
                        // Read from file store.
                        let blob_file =
                            Object::download(&bucket_name, &generate_blob_name(current_version))
                                .await
                                .expect("[indexer gcs] Failed to get file store metadata.");
                        let blob: TransactionsBlob = serde_json::from_slice(&blob_file)
                            .expect("[indexer gcs] Failed to deserialize blob.");
                        blob.transactions
                    },

                    CacheCoverageStatus::DataNotReady => {
                        sleep(Duration::from_millis(DATA_NOT_READY_SLEEP_DURATION));
                        continue;
                    },
                    CacheCoverageStatus::CacheHit => {
                        get_cache_transactions(&mut conn, current_version)
                            .await
                            .expect("[Indexer Data] Failed to get cache transactions.")
                    },
                };

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
                    chain_id,
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
                current_version += BLOB_TRANSACTION_CHUNK_SIZE;
                ma.tick_now(BLOB_TRANSACTION_CHUNK_SIZE);
                info!(
                    request_id = request_id.as_str(),
                    current_version = current_version,
                    batch_size = BLOB_TRANSACTION_CHUNK_SIZE,
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
