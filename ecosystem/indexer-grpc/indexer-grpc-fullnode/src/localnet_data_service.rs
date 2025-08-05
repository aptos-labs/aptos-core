// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{stream_coordinator::IndexerStreamCoordinator, ServiceContext};
use aptos_logger::{error, info};
use aptos_protos::{
    indexer::v1::{
        raw_data_server::RawData, EventWithMetadata, EventsResponse, GetEventsRequest,
        GetTransactionsRequest, TransactionsResponse,
    },
    internal::fullnode::v1::transactions_from_node_response,
};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

// Default Values
pub const DEFAULT_NUM_RETRIES: usize = 3;
pub const RETRY_TIME_MILLIS: u64 = 100;
const TRANSACTION_CHANNEL_SIZE: usize = 35;

type TransactionResponseStream =
    Pin<Box<dyn Stream<Item = Result<TransactionsResponse, Status>> + Send>>;

pub struct LocalnetDataService {
    pub service_context: ServiceContext,
}

/// External service on the fullnode is for testing/local development only.
/// Performance is not optimized, e.g., single-threaded.
/// NOTE: code is duplicated from fullnode_data_service.rs with some minor changes.
#[tonic::async_trait]
impl RawData for LocalnetDataService {
    type GetEventsStream = Pin<Box<dyn Stream<Item = Result<EventsResponse, Status>> + Send>>;
    type GetTransactionsStream = TransactionResponseStream;

    async fn get_transactions(
        &self,
        req: Request<GetTransactionsRequest>,
    ) -> Result<Response<Self::GetTransactionsStream>, Status> {
        // Some node metadata
        let context = self.service_context.context.clone();
        let r = req.into_inner();
        let starting_version = r.starting_version.expect("Starting version must be set");
        let ending_version = if let Some(count) = r.transactions_count {
            starting_version.saturating_add(count)
        } else {
            u64::MAX
        };
        let processor_batch_size = self.service_context.processor_batch_size;
        let output_batch_size = self.service_context.output_batch_size;
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
                ending_version,
                // Performance is not important for raw data, and to make sure data is in order,
                // single thread is used.
                1,
                processor_batch_size,
                output_batch_size,
                tx.clone(),
            );
            while coordinator.current_version < coordinator.end_version {
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

                let response = response.map(|t| TransactionsResponse {
                    chain_id: Some(ledger_chain_id as u64),
                    transactions: match t.response.expect("Response must be set") {
                        transactions_from_node_response::Response::Data(transaction_output) => {
                            let mut transactions = transaction_output.transactions;
                            let current_transactions_count = transactions.len() as u64;
                            if let Some(count) = response_transactions_count.as_mut() {
                                transactions =
                                    transactions.into_iter().take(*count as usize).collect();
                                *count = count.saturating_sub(current_transactions_count);
                            }
                            transactions
                        },
                        _ => panic!("Unexpected response type."),
                    },
                    processed_range: None,
                });
                match external_service_tx.send(response).await {
                    Ok(_) => {},
                    Err(e) => {
                        aptos_logger::warn!(
                            "[indexer-grpc] Unable to send end batch status: {:?}",
                            e
                        );
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

    async fn get_events(
        &self,
        req: Request<GetEventsRequest>,
    ) -> Result<Response<Self::GetEventsStream>, Status> {
        // Convert GetEventsRequest to GetTransactionsRequest
        let events_req = req.into_inner();
        let transactions_req = Request::new(GetTransactionsRequest {
            starting_version: events_req.starting_version,
            transactions_count: events_req.transactions_count,
            batch_size: events_req.batch_size,
            transaction_filter: events_req.transaction_filter,
        });

        // Get the response from get_transactions
        let transactions_response = self.get_transactions(transactions_req).await?;
        let transactions_stream = transactions_response.into_inner();

        // Transform transaction responses to event responses
        let events_stream = transactions_stream.map(|result| {
            result.map(|transactions_response| {
                let mut events = Vec::new();

                for transaction in transactions_response.transactions {
                    if let Some(ref txn_info) = transaction.info {
                        let timestamp = transaction.timestamp;
                        let version = transaction.version;
                        let hash = txn_info.hash.clone();
                        let success = txn_info.success;
                        let vm_status = txn_info.vm_status.clone();
                        let block_height = transaction.block_height;

                        // Extract events from transaction data
                        if let Some(txn_data) = &transaction.txn_data {
                            use aptos_protos::transaction::v1::transaction::TxnData;
                            let transaction_events = match txn_data {
                                TxnData::User(user_txn) => &user_txn.events,
                                TxnData::Genesis(genesis_txn) => &genesis_txn.events,
                                TxnData::BlockMetadata(block_meta_txn) => &block_meta_txn.events,
                                TxnData::StateCheckpoint(_) => continue, // No events
                                TxnData::Validator(validator_txn) => &validator_txn.events,
                                TxnData::BlockEpilogue(_) => continue, // No events
                            };

                            for event in transaction_events {
                                events.push(EventWithMetadata {
                                    event: Some(event.clone()),
                                    timestamp,
                                    version,
                                    hash: hash.clone(),
                                    success,
                                    vm_status: vm_status.clone(),
                                    block_height,
                                });
                            }
                        }
                    }
                }

                EventsResponse {
                    events,
                    chain_id: transactions_response.chain_id,
                    processed_range: transactions_response.processed_range,
                }
            })
        });

        let response = Response::new(Box::pin(events_stream) as Self::GetEventsStream);
        Ok(response)
    }
}
