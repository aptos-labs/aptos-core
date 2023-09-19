// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::stream_coordinator::IndexerStreamCoordinator;
use aptos_api::context::Context;
use aptos_config::config::NodeConfig;
use aptos_logger::{error, info};
use aptos_mempool::MempoolClientSender;
use aptos_moving_average::MovingAverage;
use aptos_protos::internal::fullnode::v1::{
    fullnode_data_server::{FullnodeData, FullnodeDataServer},
    stream_status::StatusType,
    transactions_from_node_response, GetTransactionsFromNodeRequest, StreamStatus,
    TransactionsFromNodeResponse,
};
use aptos_protos::indexer::v1::{
    GetTransactionsRequest, TransactionsResponse,
    raw_data_server::{RawDataServer, RawData}};
use aptos_storage_interface::DbReader;
use aptos_types::chain_id::ChainId;
use futures::Stream;
use std::{net::ToSocketAddrs, pin::Pin, sync::Arc};
use tokio::{runtime::Runtime, sync::mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};

// Default Values
pub const DEFAULT_NUM_RETRIES: usize = 3;
pub const RETRY_TIME_MILLIS: u64 = 100;
const TRANSACTION_CHANNEL_SIZE: usize = 35;
const DEFAULT_EMIT_SIZE: usize = 1000;

type FullnodeResponseStream =
    Pin<Box<dyn Stream<Item = Result<TransactionsFromNodeResponse, Status>> + Send>>;
type TransactionResponseStream = Pin<Box<dyn Stream<Item = Result<TransactionsResponse, Status>> + Send>>;

// The GRPC server
pub struct FullnodeDataService {
    pub context: Arc<Context>,
    pub processor_task_count: u16,
    pub processor_batch_size: u16,
    pub output_batch_size: u16,
}

// TODO: move it to data service.
pub struct RawDataService {
    pub context: Arc<Context>,
    pub processor_task_count: u16,
    pub processor_batch_size: u16,
    pub output_batch_size: u16,
}

/// Creates a runtime which creates a thread pool which sets up the grpc streaming service
/// Returns corresponding Tokio runtime
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
) -> Option<Runtime> {
    if !config.indexer_grpc.enabled {
        return None;
    }

    let runtime = aptos_runtimes::spawn_named_runtime("indexer-grpc".to_string(), None);

    let node_config = config.clone();

    // We have defaults for these so they should all return something nonnull so unwrap is safe here
    let processor_task_count = node_config.indexer_grpc.processor_task_count.unwrap();
    let processor_batch_size = node_config.indexer_grpc.processor_batch_size.unwrap();
    let output_batch_size = node_config.indexer_grpc.output_batch_size.unwrap();
    let address = node_config.indexer_grpc.address.clone().unwrap();

    runtime.spawn(async move {
        let context = Arc::new(Context::new(chain_id, db, mp_sender, node_config));
        let server = FullnodeDataService {
            context: context.clone(),
            processor_task_count,
            processor_batch_size,
            output_batch_size,
        };
        let indexer_server = RawDataService {
            context,
            processor_task_count,
            processor_batch_size,
            output_batch_size,
        };

        Server::builder()
            .http2_keepalive_interval(Some(std::time::Duration::from_secs(60)))
            .http2_keepalive_timeout(Some(std::time::Duration::from_secs(5)))
            .add_service(FullnodeDataServer::new(server))
            .add_service(RawDataServer::new(indexer_server))
            // Make port into a config
            .serve(address.to_socket_addrs().unwrap().next().unwrap())
            .await
            .unwrap();
        info!(address = address, "[indexer-grpc] Started GRPC server");
    });
    Some(runtime)
}

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
        let processor_task_count = self.processor_task_count;
        let processor_batch_size = self.processor_batch_size;
        let output_batch_size = self.output_batch_size;

        // Some node metadata
        let context = self.context.clone();
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
                    info!("[indexer-grpc] Init connection");
                },
                Err(_) => {
                    panic!("[indexer-grpc] Unable to initialize stream");
                },
            }
            let mut base: u64 = 0;
            loop {
                // Processes and sends batch of transactions to client
                let results = coordinator.process_next_batch().await;
                let max_version = match IndexerStreamCoordinator::get_max_batch_version(results) {
                    Ok(max_version) => max_version,
                    Err(e) => {
                        error!("[indexer-grpc] Error sending to stream: {}", e);
                        break;
                    },
                };
                // send end batch message (each batch) upon success of the entire batch
                // client can use the start and end version to ensure that there are no gaps
                // end loop if this message fails to send because otherwise the client can't validate
                let batch_end_status = get_status(
                    StatusType::BatchEnd,
                    coordinator.current_version,
                    Some(max_version),
                    ledger_chain_id,
                );
                match tx.send(Result::<_, Status>::Ok(batch_end_status)).await {
                    Ok(_) => {
                        // tps logging
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
                        aptos_logger::warn!("[indexer-grpc] Unable to send end batch status");
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

/// External service on the fullnode is for testing/local development only.
/// Performance is not optimized, e.g., single-threaded.
#[tonic::async_trait]
impl RawData for RawDataService {
    type GetTransactionsStream = TransactionResponseStream;

    async fn get_transactions(
        &self,
        req: Request<GetTransactionsRequest>,
    ) -> Result<Response<Self::GetTransactionsStream>, Status> {
        // Some node metadata
        let context = self.context.clone();
        let r = req.into_inner();
        let starting_version = r.starting_version.expect("Starting version must be set");
        let processor_batch_size = self.processor_batch_size;
        let output_batch_size = self.output_batch_size;
        let ledger_chain_id = context.chain_id().id();
        let transactions_count = r.transactions_count;
        // Creates a channel to send the stream to the client
        let (tx, mut rx) = mpsc::channel(TRANSACTION_CHANNEL_SIZE);
        let (external_service_tx, external_service_rx) = mpsc::channel(TRANSACTION_CHANNEL_SIZE);

        tokio::spawn(async move {
            // Initialize the coordinator that tracks starting version and processes transactions
            let mut coordinator = IndexerStreamCoordinator::new(
                context,
                starting_version,
                // Performance is not important for raw data, and to make sure data is in order,
                // single thread is used.
                1,
                processor_batch_size,
                output_batch_size,
                tx.clone(),
            );
            loop {
                // Processes and sends batch of transactions to client
                let results = coordinator.process_next_batch().await;
                let max_version = match IndexerStreamCoordinator::get_max_batch_version(results) {
                    Ok(max_version) => max_version,
                    Err(e) => {
                        error!("[indexer-grpc] Error sending to stream: {}", e);
                        break;
                    },
                };
                coordinator.current_version = max_version + 1;
            }
        });
        tokio::spawn(async move {
            let mut response_transactions_count = transactions_count;
            while let Some(response) = rx.recv().await {
                if let Some(count) = response_transactions_count.as_ref() {
                    if *count == 0 {
                        break;
                    }
                }

                let response = response.map(|t| {
                    TransactionsResponse {
                        chain_id: Some(ledger_chain_id as u64),
                        transactions: match t.response.expect("Response must be set") {
                            transactions_from_node_response::Response::Data(transaction_output) => {
                                let mut transactions = transaction_output.transactions;
                                let current_transactions_count = transactions.len() as u64;
                                if let Some(count) = response_transactions_count.as_mut() {
                                    transactions = transactions.into_iter().take(*count as usize).collect();
                                    *count = count.saturating_sub(current_transactions_count);
                                }
                                transactions
                            },
                            _ => panic!("Unexpected response type."),
                        }
                    }
                });
                match external_service_tx.send(response).await {
                    Ok(_) => {},
                    Err(e) => {
                        aptos_logger::warn!("[indexer-grpc] Unable to send end batch status: {:?}", e);
                        break;
                    },
                }
            }
        });

        let output_stream = ReceiverStream::new(external_service_rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::GetTransactionsStream
        ))
    }
}
