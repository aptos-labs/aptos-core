// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{create_grpc_client, get_ttl_in_seconds, IndexerGrpcCacheWorkerConfig};
use aptos_indexer_grpc_utils::update_cache_latest_version;
use aptos_logger::{error, info};
use aptos_moving_average::MovingAverage;
use aptos_protos::datastream::v1::{
    self as datastream, RawDatastreamRequest, RawDatastreamResponse,
};
use futures::{self, StreamExt};
use redis::{Commands, ConnectionLike};
pub struct Worker {
    redis_client: redis::Client,
    chain_id: u32,
    current_version: u64,
    grpc_address: String,
}

pub(crate) enum WorkerError {
    // Transient error; worker should retry.
    GrpcError(String),
    // Fatal error; worker should be restarted.
    FatalError(String),
}

impl Worker {
    pub async fn new(config: IndexerGrpcCacheWorkerConfig) -> Self {
        let redis_client = redis::Client::open(format!("redis://{}", config.redis_address))
            .expect("Create redis client failed.");
        Self {
            redis_client,
            chain_id: config.chain_id,
            current_version: 0,
            grpc_address: format!("http://{}", config.indexer_address),
        }
    }

    pub async fn run(&mut self) {
        // Re-connect if lost.
        // TODO: Add a restart from file store.
        // TODO: fix the chain id verification.
        let mut conn = self.redis_client.get_connection().unwrap();
        let chain_id_exists: bool = conn.exists("chain_id").unwrap();
        if !chain_id_exists {
            conn.set::<&str, u32, ()>("chain_id", self.chain_id)
                .unwrap();
        }

        loop {
            let conn = self.redis_client.get_connection().unwrap();
            let mut rpc_client = create_grpc_client(self.grpc_address.clone()).await;
            let request = tonic::Request::new(RawDatastreamRequest {
                starting_version: self.current_version,
            });
            let response = rpc_client.raw_datastream(request).await.unwrap();

            match self
                .process_streaming_response(self.current_version, response.into_inner(), conn)
                .await
            {
                Err(WorkerError::FatalError(e)) => {
                    panic!("[Indexer Cache] Fatal Error: {}", e);
                },
                _ => {
                    // If streaming ends, try to reconnect.
                    error!("[Indexer Cache] Indexer grpc connection lost. Reconnecting...");
                },
            }
        }
    }

    /// Function to process streaming response from datastream.
    /// It accepts a starting version, a response stream, and a function to process each transaction.
    pub(crate) async fn process_streaming_response(
        &mut self,
        starting_version: u64,
        mut resp_stream: impl futures_core::Stream<Item = Result<RawDatastreamResponse, tonic::Status>>
            + std::marker::Unpin,
        mut conn: impl ConnectionLike,
    ) -> Result<(), WorkerError> {
        let mut ma = MovingAverage::new(10_000);
        let mut init_signal_received = false;
        let mut transaction_count = 0;
        let mut current_version = starting_version;

        while let Some(received) = resp_stream.next().await {
            let received = match received {
                Ok(r) => r,
                Err(e) => {
                    // If the connection is lost, reconnect.
                    return Err(WorkerError::GrpcError(format!(
                        "[Indexer Cache] Indexer grpc connection lost. Reconnecting...{:?}",
                        e
                    )));
                },
            };
            // Verify chain id just in case the server is updated in the middle of running.
            assert_eq!(self.chain_id, received.chain_id);
            match received.response.unwrap() {
                datastream::raw_datastream_response::Response::Status(status) => {
                    match status.r#type {
                        0 => {
                            if init_signal_received {
                                return Err(WorkerError::GrpcError(
                                    "[Indexer Cache] Received multiple init signals.".to_string(),
                                ));
                            }
                            init_signal_received = true;
                            // If requested starting version doesn't match, restart.
                            if current_version != status.start_version {
                                {
                                    return Err(WorkerError::FatalError(
                                        "[Indexer Cache] Requested starting version doesn't match. Restarting...".to_string()
                                    ));
                                }
                            }
                        },
                        1 => {
                            assert_eq!(
                                current_version + transaction_count,
                                status.end_version.expect("End version exists.") + 1
                            );
                            current_version = status.end_version.expect("End version exists.") + 1;
                            transaction_count = 0;
                        },
                        _ => {
                            // There might be protobuf inconsistency between server and client.
                            // Panic to block running.
                            return Err(WorkerError::FatalError(
                                "[Indexer Cache] Unknown status type.".to_string(),
                            ));
                        },
                    }
                },
                datastream::raw_datastream_response::Response::Data(data) => {
                    let transaction_len = data.transactions.len();

                    let batch_start_version = data.transactions.as_slice().first().unwrap().version;
                    let batch_end_version = data.transactions.as_slice().last().unwrap().version;

                    for e in data.transactions {
                        let version = e.version;
                        let timestamp_in_seconds = match e.timestamp {
                            Some(t) => t.seconds,
                            None => 0,
                        };
                        conn.set_ex::<String, String, ()>(
                            version.to_string(),
                            e.encoded_proto_data.to_string(),
                            get_ttl_in_seconds(timestamp_in_seconds as u64) as usize,
                        )
                        .unwrap();
                    }
                    update_cache_latest_version(&mut conn, batch_end_version)
                        .await
                        .expect("Update cache latest version failed.");
                    self.current_version = batch_end_version;
                    ma.tick_now(transaction_len as u64);
                    transaction_count += transaction_len as u64;
                    info!(
                        batch_start_version = batch_start_version,
                        batch_end_version = batch_end_version,
                        tps = (ma.avg() * 1000.0) as u64,
                        "[Indexer Cache] Sent batch successfully"
                    );
                },
            };
        }
        Ok(())
    }
}
