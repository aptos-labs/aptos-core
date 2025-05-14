// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{CHANNEL_SIZE, LATENCY_MS},
    stream_coordinator::IndexerStreamCoordinator,
    ServiceContext,
};
use aptos_indexer_grpc_utils::{
    counters::{log_grpc_step_fullnode, IndexerGrpcStep},
    timestamp_now_proto,
};
use aptos_logger::{error, info};
use aptos_moving_average::MovingAverage;
use aptos_protos::{
    indexer::v1::FullnodeInfo,
    internal::fullnode::v1::{
        fullnode_data_server::FullnodeData, stream_status::StatusType,
        transactions_from_node_response, GetTransactionsFromNodeRequest, PingFullnodeRequest,
        PingFullnodeResponse, StreamStatus, TransactionsFromNodeResponse,
    },
};
use futures::Stream;
use std::{
    pin::Pin,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

pub struct FullnodeDataService {
    pub service_context: ServiceContext,
}

type FullnodeResponseStream =
    Pin<Box<dyn Stream<Item = Result<TransactionsFromNodeResponse, Status>> + Send>>;

// Default Values
pub const DEFAULT_NUM_RETRIES: usize = 3;
pub const RETRY_TIME_MILLIS: u64 = 100;
const TRANSACTION_CHANNEL_SIZE: usize = 35;
const DEFAULT_EMIT_SIZE: usize = 1000;
const SERVICE_TYPE: &str = "indexer_fullnode";

#[tonic::async_trait]
impl FullnodeData for FullnodeDataService {
    type GetTransactionsFromNodeStream = FullnodeResponseStream;

    /// This function is required by the GRPC tonic server. It basically handles the request.
    /// Given we want to persist the stream for better performance, our approach is that when
    /// we receive a request, we will return a stream. Then as we process transactions, we
    /// wrap those into a TransactionsResponse that we then push into the stream.
    /// There are 2 types of TransactionsResponse:
    /// Status - sends events back to the client, such as init stream and batch end
    /// Transaction - sends encoded transactions lightly wrapped
    async fn get_transactions_from_node(
        &self,
        req: Request<GetTransactionsFromNodeRequest>,
    ) -> Result<Response<Self::GetTransactionsFromNodeStream>, Status> {
        // Gets configs for the stream, partly from the request and partly from the node config
        let r = req.into_inner();
        let starting_version = r.starting_version.expect("Starting version must be set");
        let processor_task_count = self.service_context.processor_task_count;
        let processor_batch_size = self.service_context.processor_batch_size;
        let output_batch_size = self.service_context.output_batch_size;

        // Some node metadata
        let context = self.service_context.context.clone();
        let ledger_chain_id = context.chain_id().id();

        // Creates a channel to send the stream to the client
        let (tx, rx) = mpsc::channel(TRANSACTION_CHANNEL_SIZE);

        // Creates a moving average to track tps
        let mut ma = MovingAverage::new(10_000);

        // This is the main thread handling pushing to the stream
        tokio::spawn(async move {
            // Initialize the coordinator that tracks starting version and processes transactions
            let mut coordinator = IndexerStreamCoordinator::new(
                context,
                starting_version,
                processor_task_count,
                processor_batch_size,
                output_batch_size,
                tx.clone(),
            );
            // Sends init message (one time per request) to the client in the with chain id and starting version. Basically a handshake
            let init_status = get_status(StatusType::Init, starting_version, None, ledger_chain_id);
            match tx.send(Result::<_, Status>::Ok(init_status)).await {
                Ok(_) => {
                    // TODO: Add request details later
                    info!(
                        start_version = starting_version,
                        chain_id = ledger_chain_id,
                        service_type = SERVICE_TYPE,
                        "[Indexer Fullnode] Init connection"
                    );
                },
                Err(_) => {
                    panic!("[Indexer Fullnode] Unable to initialize stream");
                },
            }
            let mut base: u64 = 0;
            loop {
                let start_time = std::time::Instant::now();
                // Processes and sends batch of transactions to client
                let results = coordinator.process_next_batch().await;
                if results.is_empty() {
                    info!(
                        start_version = starting_version,
                        chain_id = ledger_chain_id,
                        "[Indexer Fullnode] Client disconnected."
                    );
                    break;
                }
                let max_version = match IndexerStreamCoordinator::get_max_batch_version(results) {
                    Ok(max_version) => max_version,
                    Err(e) => {
                        error!("[Indexer Fullnode] Error sending to stream: {}", e);
                        break;
                    },
                };
                let highest_known_version = coordinator.highest_known_version;

                // send end batch message (each batch) upon success of the entire batch
                // client can use the start and end version to ensure that there are no gaps
                // end loop if this message fails to send because otherwise the client can't validate
                let batch_end_status = get_status(
                    StatusType::BatchEnd,
                    coordinator.current_version,
                    Some(max_version),
                    ledger_chain_id,
                );
                let channel_size = TRANSACTION_CHANNEL_SIZE - tx.capacity();
                CHANNEL_SIZE
                    .with_label_values(&["2"])
                    .set(channel_size as i64);
                match tx.send(Result::<_, Status>::Ok(batch_end_status)).await {
                    Ok(_) => {
                        // tps logging
                        let new_base: u64 = ma.sum() / (DEFAULT_EMIT_SIZE as u64);
                        ma.tick_now(max_version - coordinator.current_version + 1);
                        if base != new_base {
                            base = new_base;

                            log_grpc_step_fullnode(
                                IndexerGrpcStep::FullnodeProcessedBatch,
                                Some(coordinator.current_version as i64),
                                Some(max_version as i64),
                                None,
                                Some(highest_known_version as i64),
                                Some(ma.avg() * 1000.0),
                                Some(start_time.elapsed().as_secs_f64()),
                                Some((max_version - coordinator.current_version + 1) as i64),
                            );
                        }
                    },
                    Err(_) => {
                        aptos_logger::warn!("[Indexer Fullnode] Unable to send end batch status");
                        break;
                    },
                }
                coordinator.current_version = max_version + 1;
            }
        });
        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::GetTransactionsFromNodeStream
        ))
    }

    async fn ping(
        &self,
        _request: Request<PingFullnodeRequest>,
    ) -> Result<Response<PingFullnodeResponse>, Status> {
        let timestamp = timestamp_now_proto();
        let known_latest_version = self
            .service_context
            .context
            .db
            .get_synced_version()
            .map_err(|e| Status::internal(format!("{e}")))?;

        let table_info_version = self
            .service_context
            .context
            .indexer_reader
            .as_ref()
            .and_then(|r| r.get_latest_table_info_ledger_version().ok().flatten());

        if known_latest_version.is_some() && table_info_version.is_some() {
            let version = std::cmp::min(known_latest_version.unwrap(), table_info_version.unwrap());
            if let Ok(timestamp_us) = self.service_context.context.db.get_block_timestamp(version) {
                let latency = SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
                    - Duration::from_micros(timestamp_us);
                LATENCY_MS.set(latency.as_millis() as i64);
            }
        }

        let info = FullnodeInfo {
            chain_id: self.service_context.context.chain_id().id() as u64,
            timestamp: Some(timestamp),
            known_latest_version,
        };
        let response = PingFullnodeResponse { info: Some(info) };
        Ok(Response::new(response))
    }
}

pub fn get_status(
    status_type: StatusType,
    start_version: u64,
    end_version: Option<u64>,
    ledger_chain_id: u8,
) -> TransactionsFromNodeResponse {
    TransactionsFromNodeResponse {
        response: Some(transactions_from_node_response::Response::Status(
            StreamStatus {
                r#type: status_type as i32,
                start_version,
                end_version,
            },
        )),
        chain_id: ledger_chain_id as u32,
    }
}
