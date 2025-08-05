// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{stream_coordinator::IndexerStreamCoordinator, ServiceContext};
use aptos_indexer_grpc_data_service_v2::websocket::{
    WebSocketGetEventsRequest, WebSocketGetTransactionsRequest, WebSocketMessage,
};
use aptos_logger::{error, info, warn};
use aptos_protos::{
    indexer::v1::{EventsResponse, TransactionsResponse},
    internal::fullnode::v1::transactions_from_node_response,
};
use axum::{
    extract::{ws::Message, ws::WebSocket, WebSocketUpgrade, State},
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;

// Default Values
pub const DEFAULT_NUM_RETRIES: usize = 3;
pub const RETRY_TIME_MILLIS: u64 = 100;
const TRANSACTION_CHANNEL_SIZE: usize = 35;



/// WebSocket handler for transactions stream
pub async fn websocket_transactions_handler(
    ws: WebSocketUpgrade,
    State(service_context): State<Arc<ServiceContext>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_transactions_websocket(socket, service_context))
}

/// WebSocket handler for events stream
pub async fn websocket_events_handler(
    ws: WebSocketUpgrade,
    State(service_context): State<Arc<ServiceContext>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_events_websocket(socket, service_context))
}

async fn handle_transactions_websocket(socket: WebSocket, service_context: Arc<ServiceContext>) {
    let (mut sender, mut receiver) = socket.split();

    // Wait for the initial request
    if let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<WebSocketGetTransactionsRequest>(&text) {
                    Ok(ws_request) => {
                        info!("Starting transactions WebSocket stream: {:?}", ws_request);

                        // Extract request parameters
                        let context = service_context.context.clone();
                        let starting_version = ws_request.starting_version.unwrap_or(0);
                        let ending_version = if let Some(count) = ws_request.transactions_count {
                            starting_version.saturating_add(count)
                        } else {
                            u64::MAX
                        };
                        let processor_batch_size = service_context.processor_batch_size;
                        let output_batch_size = service_context.output_batch_size;
                        let ledger_chain_id = context.chain_id().id();
                        let transactions_count = ws_request.transactions_count;

                        // todo implement this
                        // Note: transaction_filter is not implemented in localnet for simplicity.
                        // This is intended for testing/development use only.

                        // Create channels for streaming
                        let (tx, mut rx) = mpsc::channel(TRANSACTION_CHANNEL_SIZE);

                        // Spawn coordinator task
                        tokio::spawn(async move {
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
                                let results = coordinator.process_next_batch().await;
                                if results.is_empty() {
                                    info!(
                                        start_version = starting_version,
                                        chain_id = ledger_chain_id,
                                        "[Indexer Fullnode WebSocket] Client disconnected."
                                    );
                                    break;
                                }
                                let max_version = match IndexerStreamCoordinator::get_max_batch_version(results) {
                                    Ok(max_version) => max_version,
                                    Err(e) => {
                                        error!("[indexer-grpc-ws] Error sending to stream: {}", e);
                                        break;
                                    },
                                };
                                coordinator.current_version = max_version + 1;
                            }
                        });

                        // Stream responses back to WebSocket
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
                                    _ => {
                                        error!("Unexpected response type");
                                        vec![]
                                    },
                                },
                                processed_range: None,
                            });

                            let ws_message = match response {
                                Ok(transactions_response) => WebSocketMessage::TransactionsResponse(transactions_response),
                                Err(e) => {
                                    error!("Error processing transaction response: {}", e);
                                    WebSocketMessage::Error {
                                        message: format!("Processing error: {}", e),
                                    }
                                },
                            };

                            let json_msg = match serde_json::to_string(&ws_message) {
                                Ok(json) => json,
                                Err(e) => {
                                    error!("Failed to serialize transactions response: {}", e);
                                    let error_msg = WebSocketMessage::Error {
                                        message: format!("Serialization error: {}", e),
                                    };
                                    serde_json::to_string(&error_msg).unwrap_or_else(|_| "{}".to_string())
                                },
                            };

                            if let Err(e) = sender.send(Message::Text(json_msg)).await {
                                warn!("WebSocket send error: {}", e);
                                break;
                            }
                        }

                        // Send stream end message
                        let end_msg = WebSocketMessage::StreamEnd;
                        let end_json = serde_json::to_string(&end_msg)
                            .unwrap_or_else(|_| "{}".to_string());
                        let _ = sender.send(Message::Text(end_json)).await;
                    },
                    Err(e) => {
                        error!("Failed to parse WebSocket request: {}", e);
                        let error_msg = WebSocketMessage::Error {
                            message: format!("Invalid request format: {}", e),
                        };
                        let error_json =
                            serde_json::to_string(&error_msg).unwrap_or_else(|_| "{}".to_string());
                        let _ = sender.send(Message::Text(error_json)).await;
                    },
                }
            },
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed by client");
            },
            Err(e) => {
                error!("WebSocket error: {}", e);
            },
            _ => {
                warn!("Unexpected WebSocket message type");
            },
        }
    }
}

async fn handle_events_websocket(socket: WebSocket, service_context: Arc<ServiceContext>) {
    let (mut sender, mut receiver) = socket.split();

    // Wait for the initial request
    if let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<WebSocketGetEventsRequest>(&text) {
                    Ok(ws_request) => {
                        info!("Starting events WebSocket stream: {:?}", ws_request);

                        // Extract request parameters
                        let starting_version = ws_request.starting_version.unwrap_or(0);
                        let transactions_count = ws_request.transactions_count;

                        // Note: transaction_filter is not implemented in localnet for simplicity.
                        // This is intended for testing/development use only.

                        // Use the existing transaction processing logic but extract events
                        let context = service_context.context.clone();
                        let ending_version = if let Some(count) = transactions_count {
                            starting_version.saturating_add(count)
                        } else {
                            u64::MAX
                        };
                        let processor_batch_size = service_context.processor_batch_size;
                        let output_batch_size = service_context.output_batch_size;
                        let ledger_chain_id = context.chain_id().id();

                        // Create channels for streaming
                        let (tx, mut rx) = mpsc::channel(TRANSACTION_CHANNEL_SIZE);

                        // Spawn coordinator task
                        tokio::spawn(async move {
                            let mut coordinator = IndexerStreamCoordinator::new(
                                context,
                                starting_version,
                                ending_version,
                                1,
                                processor_batch_size,
                                output_batch_size,
                                tx.clone(),
                            );
                            while coordinator.current_version < coordinator.end_version {
                                let results = coordinator.process_next_batch().await;
                                if results.is_empty() {
                                    info!(
                                        start_version = starting_version,
                                        chain_id = ledger_chain_id,
                                        "[Indexer Fullnode WebSocket Events] Client disconnected."
                                    );
                                    break;
                                }
                                let max_version = match IndexerStreamCoordinator::get_max_batch_version(results) {
                                    Ok(max_version) => max_version,
                                    Err(e) => {
                                        error!("[indexer-grpc-ws-events] Error sending to stream: {}", e);
                                        break;
                                    },
                                };
                                coordinator.current_version = max_version + 1;
                            }
                        });

                        // Transform transaction responses to event responses
                        let mut response_transactions_count = transactions_count;
                        while let Some(response) = rx.recv().await {
                            if let Some(count) = response_transactions_count.as_ref() {
                                if *count == 0 {
                                    break;
                                }
                            }

                            let events_response = match response {
                                Ok(transactions_response) => {
                                    match transactions_response.response.expect("Response must be set") {
                                        transactions_from_node_response::Response::Data(transaction_output) => {
                                            let mut events = Vec::new();
                                            let mut transactions = transaction_output.transactions;
                                            let current_transactions_count = transactions.len() as u64;

                                            if let Some(count) = response_transactions_count.as_mut() {
                                                transactions = transactions.into_iter().take(*count as usize).collect();
                                                *count = count.saturating_sub(current_transactions_count);
                                            }

                                            // Extract events from transactions
                                            for transaction in transactions {
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
                                                            // todo: the event data is just a string, it should be proper json embedded
                                                            // within the response.
                                                            events.push(aptos_protos::indexer::v1::EventWithMetadata {
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

                                            Ok(EventsResponse {
                                                events,
                                                chain_id: Some(ledger_chain_id as u64),
                                                processed_range: None,
                                            })
                                        },
                                        _ => {
                                            Err("Unexpected response type".to_string())
                                        },
                                    }
                                },
                                Err(e) => Err(format!("Error processing transactions: {}", e)),
                            };

                            let ws_message = match events_response {
                                Ok(response) => WebSocketMessage::EventsResponse(response),
                                Err(e) => {
                                    error!("Error processing events response: {}", e);
                                    WebSocketMessage::Error {
                                        message: format!("Processing error: {}", e),
                                    }
                                },
                            };

                            let json_msg = match serde_json::to_string(&ws_message) {
                                Ok(json) => json,
                                Err(e) => {
                                    error!("Failed to serialize events response: {}", e);
                                    let error_msg = WebSocketMessage::Error {
                                        message: format!("Serialization error: {}", e),
                                    };
                                    serde_json::to_string(&error_msg).unwrap_or_else(|_| "{}".to_string())
                                },
                            };

                            if let Err(e) = sender.send(Message::Text(json_msg)).await {
                                warn!("WebSocket send error: {}", e);
                                break;
                            }
                        }

                        // Send stream end message
                        let end_msg = WebSocketMessage::StreamEnd;
                        let end_json = serde_json::to_string(&end_msg)
                            .unwrap_or_else(|_| "{}".to_string());
                        let _ = sender.send(Message::Text(end_json)).await;
                    },
                    Err(e) => {
                        error!("Failed to parse WebSocket events request: {}", e);
                        let error_msg = WebSocketMessage::Error {
                            message: format!("Invalid request format: {}", e),
                        };
                        let error_json =
                            serde_json::to_string(&error_msg).unwrap_or_else(|_| "{}".to_string());
                        let _ = sender.send(Message::Text(error_json)).await;
                    },
                }
            },
            Ok(Message::Close(_)) => {
                info!("WebSocket events connection closed by client");
            },
            Err(e) => {
                error!("WebSocket events error: {}", e);
            },
            _ => {
                warn!("Unexpected WebSocket message type");
            },
        }
    }
}