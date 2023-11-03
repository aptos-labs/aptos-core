// Copyright © Aptos Foundation

use crate::{stream_coordinator::IndexerStreamCoordinator, ServiceContext};
use aptos_logger::{error, info};
use aptos_moving_average::MovingAverage;
use aptos_protos::internal::fullnode::v1::{
    fullnode_data_server::FullnodeData, stream_status::StatusType, transactions_from_node_response,
    GetTransactionsFromNodeRequest, StreamStatus, TransactionsFromNodeResponse,
};
use deadpool_postgres::Pool;
use futures::Stream;
use std::{pin::Pin, sync::Arc};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

pub struct FullnodeDataService {
    pub service_context: ServiceContext,
    pub pool: Arc<Pool>,
}

type FullnodeResponseStream =
    Pin<Box<dyn Stream<Item = Result<TransactionsFromNodeResponse, Status>> + Send>>;

// Default Values
pub const DEFAULT_NUM_RETRIES: usize = 3;
pub const RETRY_TIME_MILLIS: u64 = 100;
const TRANSACTION_CHANNEL_SIZE: usize = 35;
const DEFAULT_EMIT_SIZE: usize = 1000;

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
        let pool = self.pool.clone();
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
                // let results = coordinator.process_next_batch(self.pool).await;
                let results = coordinator.process_next_batch(pool).await;
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
