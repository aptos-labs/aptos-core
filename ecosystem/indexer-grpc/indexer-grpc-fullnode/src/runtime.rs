// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::stream_coordinator::IndexerStreamCoordinator;
use aptos_api::context::Context;
use aptos_config::config::NodeConfig;
use aptos_logger::{error, info};
use aptos_mempool::MempoolClientSender;
use aptos_protos::datastream::v1::{
    indexer_stream_server::{IndexerStream, IndexerStreamServer},
    raw_datastream_response,
    stream_status::StatusType,
    RawDatastreamRequest, RawDatastreamResponse, StreamStatus,
};
use aptos_storage_interface::DbReader;
use aptos_types::chain_id::ChainId;
use futures::Stream;
use moving_average::MovingAverage;
use std::{net::ToSocketAddrs, pin::Pin, sync::Arc};
use tokio::{
    runtime::{Builder, Runtime},
    sync::mpsc,
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};

// Default Values
pub const DEFAULT_NUM_RETRIES: usize = 3;
pub const RETRY_TIME_MILLIS: u64 = 300;
const TRANSACTION_CHANNEL_SIZE: usize = 35;
const DEFAULT_EMIT_SIZE: usize = 1000;

type ResponseStream = Pin<Box<dyn Stream<Item = Result<RawDatastreamResponse, Status>> + Send>>;

// The GRPC server
pub struct IndexerStreamService {
    pub context: Arc<Context>,
}

/// Creates a runtime which creates a thread pool which pushes firehose of block protobuf to SF endpoint
/// Returns corresponding Tokio runtime
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
) -> Option<anyhow::Result<Runtime>> {
    if !config.indexer_grpc.enabled {
        return None;
    }

    let runtime = Builder::new_multi_thread()
        .thread_name("indexer-grpc")
        .disable_lifo_slot()
        .enable_all()
        .build()
        .expect("[indexer-grpc] failed to create runtime");

    let node_config = config.clone();

    runtime.spawn(async move {
        let context = Arc::new(Context::new(chain_id, db, mp_sender, node_config));
        let server = IndexerStreamService { context };

        Server::builder()
            .add_service(IndexerStreamServer::new(server))
            // Make port into a config
            .serve("0.0.0.0:50051".to_socket_addrs().unwrap().next().unwrap())
            .await
            .unwrap();
        info!("[indexer-grpc] Started GRPC server at 50051");
    });
    Some(Ok(runtime))
}

#[tonic::async_trait]
impl IndexerStream for IndexerStreamService {
    type RawDatastreamStream = ResponseStream;

    async fn raw_datastream(
        &self,
        req: Request<RawDatastreamRequest>,
    ) -> Result<Response<Self::RawDatastreamStream>, Status> {
        let r = req.into_inner();
        let starting_version = r.starting_version;
        let processor_task_count = r.processor_task_count as u16;
        let processor_batch_size = r.processor_batch_size as u16;
        let output_batch_size = r.output_batch_size as u16;
        let chain_id = r.chain_id as u8;

        let (tx, rx) = mpsc::channel(TRANSACTION_CHANNEL_SIZE);
        let context = self.context.clone();
        let mut ma = MovingAverage::new(10_000);

        tokio::spawn(async move {
            let mut coordinator = IndexerStreamCoordinator::new(
                context,
                chain_id,
                starting_version,
                processor_task_count,
                processor_batch_size,
                output_batch_size,
                tx.clone(),
            );
            let init_status = Self::get_status(StatusType::Init, starting_version, None);
            match tx.send(Result::<_, Status>::Ok(init_status)).await {
                Ok(_) => {
                    // TODO: Add request details later
                    info!("[indexer-grpc] Init connection");
                },
                Err(_) => {
                    panic!("[indexer-grpc] Unable to initialize stream");
                },
            }
            let mut base: u64 = 0;
            loop {
                let results = coordinator.process_next_batch().await;
                let mut is_error = false;
                let mut max_version = 0;
                for result in results {
                    match result {
                        Ok(end_version) => {
                            max_version = std::cmp::max(max_version, end_version);
                        },
                        Err(e) => {
                            error!("[indexer-grpc] Error sending to stream: {}", e);
                            is_error = true;
                            break;
                        },
                    }
                }
                if is_error {
                    break;
                }
                let init_status = Self::get_status(
                    StatusType::BatchEnd,
                    coordinator.current_version,
                    Some(max_version),
                );
                match tx.send(Result::<_, Status>::Ok(init_status)).await {
                    Ok(_) => {
                        let new_base: u64 = ma.sum() / (DEFAULT_EMIT_SIZE as u64);
                        ma.tick_now(max_version - coordinator.current_version + 1);
                        if base != new_base {
                            base = new_base;

                            info!(
                                batch_start_version = coordinator.current_version,
                                batch_end_version = max_version,
                                versions_processed = ma.sum(),
                                tps = (ma.avg() * 1000.0) as u64,
                                "[indexer-grpc] Sent batch successfully"
                            );
                        }
                    },
                    Err(_) => {
                        panic!("[indexer-grpc] Unable to initialize stream");
                    },
                }
                coordinator.current_version = max_version + 1;
            }
        });
        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::RawDatastreamStream
        ))
    }
}

impl IndexerStreamService {
    pub fn get_status(
        status_type: StatusType,
        start_version: u64,
        end_version: Option<u64>,
    ) -> RawDatastreamResponse {
        RawDatastreamResponse {
            response: Some(raw_datastream_response::Response::Status(StreamStatus {
                r#type: status_type as i32,
                start_version,
                end_version,
            })),
        }
    }
}
