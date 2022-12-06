// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{self, FETCHED_TRANSACTION, UNABLE_TO_FETCH_TRANSACTION},
    stream_coordinator::IndexerStreamCoordinator,
};
use anyhow::ensure;
use aptos_protos::{
    datastream::v1::{
        indexer_stream_server::{IndexerStream, IndexerStreamServer},
        raw_datastream_response, RawDatastreamRequest, RawDatastreamResponse, StreamStatus,
        TransactionOutput, TransactionsOutput,
    },
    extractor::v1 as extractor,
};
use tonic::{Request, Response, Status};

use crate::convert::convert_transaction;
use aptos_api::context::Context;
use aptos_api_types::{AsConverter, Transaction as APITransaction, TransactionOnChainData};
use aptos_config::config::NodeConfig;
use aptos_logger::{debug, error, info, sample, sample::SampleRate, warn};
use aptos_mempool::MempoolClientSender;
use aptos_types::chain_id::ChainId;
use aptos_vm::data_cache::StorageAdapterOwned;
use extractor::Transaction as TransactionPB;
use futures::{channel::mpsc::channel, Stream};
use prost::Message;
use std::{
    convert::TryInto, f32::consts::E, net::ToSocketAddrs, pin::Pin, sync::Arc, time::Duration,
};
use storage_interface::{state_view::DbStateView, DbReader};
use tokio::{
    runtime::{Builder, Runtime},
    sync::mpsc,
    time::sleep,
};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::transport::Server;

// Default Values
const DEFAULT_NUM_RETRIES: usize = 3;
const RETRY_TIME_MILLIS: u64 = 300;
const MAX_RETRY_TIME_MILLIS: u64 = 120000;
const TRANSACTION_FETCH_BATCH_SIZE: u16 = 500;
const TRANSACTION_CHANNEL_SIZE: usize = 35;

type ResponseStream = Pin<Box<dyn Stream<Item = Result<RawDatastreamResponse, Status>> + Send>>;

// The GRPC server
pub struct IndexerStreamService {
    pub context: Arc<Context>,
    // pub resolver: Arc<StorageAdapterOwned<DbStateView>>,
    // // Tracks start of a batch. The assumption is that every version before this has been processed.
    // pub current_version: u64,
    // // This is only ever used for testing
    // pub mp_sender: MempoolClientSender,
    // pub processor_batch_size: usize,
    // pub processor_task_count: usize,
    // pub highest_known_version: u64,
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
            .serve("[::1]:50051".to_socket_addrs().unwrap().next().unwrap())
            .await
            .unwrap();
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
            // send init signal, TODO: move this into helper functions
            let item = RawDatastreamResponse {
                response: Some(raw_datastream_response::Response::Status(StreamStatus {
                    r#type: 0,
                    start_version: starting_version,
                    end_version: None,
                })),
                ..RawDatastreamResponse::default()
            };
            match tx.send(Result::<_, Status>::Ok(item)).await {
                Ok(_) => {}
                Err(_) => {
                    panic!("Unable to initialize stream");
                }
            }
            loop {
                let results = coordinator.process_next_batch().await;
                let mut is_error = false;
                let mut max_version = 0;
                for result in results {
                    match result {
                        Ok(end_version) => {
                            max_version = std::cmp::max(max_version, end_version);
                        }
                        Err(e) => {
                            error!("[indexer-grpc] Error sending to stream: {}", e);
                            is_error = true;
                            break;
                        }
                    }
                }
                if is_error {
                    break;
                }
                // send end signal, TODO: move this into helper functions
                let item = RawDatastreamResponse {
                    response: Some(raw_datastream_response::Response::Status(StreamStatus {
                        r#type: 1,
                        start_version: coordinator.current_version,
                        end_version: Some(max_version),
                    })),
                    ..RawDatastreamResponse::default()
                };
                match tx.send(Result::<_, Status>::Ok(item)).await {
                    Ok(_) => {}
                    Err(_) => {
                        panic!("Unable to initialize stream");
                    }
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
