// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::service::DataServiceWrapperWrapper;
use aptos_protos::indexer::v1::{
    data_service_server::DataService, BooleanTransactionFilter, EventsResponse, GetEventsRequest,
    GetTransactionsRequest, TransactionsResponse,
};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tonic::Request;
use tracing::{error, info, warn};

/// WebSocket request for getting transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketGetTransactionsRequest {
    /// Required; start version of current stream.
    pub starting_version: Option<u64>,
    /// Optional; number of transactions to return in current stream.
    /// If not present, return an infinite stream of transactions.
    pub transactions_count: Option<u64>,
    /// Optional; number of transactions in each `TransactionsResponse` for current stream.
    /// If not present, default to 1000. If larger than 1000, request will be rejected.
    pub batch_size: Option<u64>,
    /// If provided, only transactions that match the filter will be included.
    pub transaction_filter: Option<BooleanTransactionFilter>,
}

/// WebSocket request for getting events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketGetEventsRequest {
    /// Required; start version of current stream.
    pub starting_version: Option<u64>,
    /// Optional; number of transactions to process in current stream.
    /// If not present, return an infinite stream of events.
    pub transactions_count: Option<u64>,
    /// Optional; number of events in each `EventsResponse` for current stream.
    /// If not present, default to 1000. If larger than 1000, request will be rejected.
    pub batch_size: Option<u64>,
    /// If provided, only transactions that match the filter will be included,
    /// and only events from those transactions will be returned.
    pub transaction_filter: Option<BooleanTransactionFilter>,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    #[serde(rename = "get_transactions")]
    GetTransactions(WebSocketGetTransactionsRequest),
    #[serde(rename = "get_events")]
    GetEvents(WebSocketGetEventsRequest),
    #[serde(rename = "transactions_response")]
    TransactionsResponse(TransactionsResponse),
    #[serde(rename = "events_response")]
    EventsResponse(EventsResponse),
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "stream_end")]
    StreamEnd,
}

/// WebSocket handler for transactions stream
pub async fn websocket_transactions_handler(
    ws: WebSocketUpgrade,
    State(service): State<Arc<DataServiceWrapperWrapper>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_transactions_websocket(socket, service))
}

/// WebSocket handler for events stream
pub async fn websocket_events_handler(
    ws: WebSocketUpgrade,
    State(service): State<Arc<DataServiceWrapperWrapper>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_events_websocket(socket, service))
}

async fn handle_transactions_websocket(socket: WebSocket, service: Arc<DataServiceWrapperWrapper>) {
    let (mut sender, mut receiver) = socket.split();

    // Wait for the initial request
    if let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<WebSocketGetTransactionsRequest>(&text) {
                    Ok(ws_request) => {
                        info!("Starting transactions WebSocket stream: {:?}", ws_request);

                        // Convert WebSocket request to gRPC request
                        let grpc_request = Request::new(GetTransactionsRequest {
                            starting_version: ws_request.starting_version,
                            transactions_count: ws_request.transactions_count,
                            batch_size: ws_request.batch_size,
                            transaction_filter: ws_request.transaction_filter,
                        });

                        // Get the gRPC stream
                        match service.get_transactions(grpc_request).await {
                            Ok(response) => {
                                let mut stream = response.into_inner();

                                // Stream responses back to WebSocket
                                while let Some(result) = stream.next().await {
                                    let transactions_response = match result {
                                        Ok(response) => response,
                                        Err(e) => {
                                            error!("gRPC stream error: {}", e);
                                            let error_msg = WebSocketMessage::Error {
                                                message: format!("gRPC stream error: {}", e),
                                            };
                                            let error_json = serde_json::to_string(&error_msg)
                                                .unwrap_or_else(|_| "{}".to_string());
                                            let _ = sender.send(Message::Text(error_json)).await;
                                            break;
                                        },
                                    };
                                    let ws_message = WebSocketMessage::TransactionsResponse(
                                        transactions_response,
                                    );
                                    let json_msg = match serde_json::to_string(&ws_message) {
                                        Ok(json) => json,
                                        Err(e) => {
                                            error!(
                                                "Failed to serialize transactions response: {}",
                                                e
                                            );
                                            let error_msg = WebSocketMessage::Error {
                                                message: format!("Serialization error: {}", e),
                                            };
                                            let error_json = serde_json::to_string(&error_msg)
                                                .unwrap_or_else(|_| "{}".to_string());
                                            let _ = sender.send(Message::Text(error_json)).await;
                                            break;
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
                                error!("gRPC error: {}", e);
                                let error_msg = WebSocketMessage::Error {
                                    message: format!("gRPC error: {}", e),
                                };
                                let error_json = serde_json::to_string(&error_msg)
                                    .unwrap_or_else(|_| "{}".to_string());
                                let _ = sender.send(Message::Text(error_json)).await;
                            },
                        }
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

async fn handle_events_websocket(socket: WebSocket, service: Arc<DataServiceWrapperWrapper>) {
    let (mut sender, mut receiver) = socket.split();

    // Wait for the initial request
    if let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<WebSocketGetEventsRequest>(&text) {
                    Ok(ws_request) => {
                        info!("Starting events WebSocket stream: {:?}", ws_request);

                        // Convert WebSocket request to gRPC request
                        let grpc_request = Request::new(GetEventsRequest {
                            starting_version: ws_request.starting_version,
                            transactions_count: ws_request.transactions_count,
                            batch_size: ws_request.batch_size,
                            transaction_filter: ws_request.transaction_filter,
                        });

                        // Get the gRPC stream
                        match service.get_events(grpc_request).await {
                            Ok(response) => {
                                let mut stream = response.into_inner();

                                // Stream responses back to WebSocket
                                while let Some(result) = stream.next().await {
                                    let events_response = match result {
                                        Ok(response) => response,
                                        Err(e) => {
                                            error!("gRPC stream error: {}", e);
                                            let error_msg = WebSocketMessage::Error {
                                                message: format!("gRPC stream error: {}", e),
                                            };
                                            let error_json = serde_json::to_string(&error_msg)
                                                .unwrap_or_else(|_| "{}".to_string());
                                            let _ = sender.send(Message::Text(error_json)).await;
                                            break;
                                        },
                                    };
                                    let ws_message =
                                        WebSocketMessage::EventsResponse(events_response);
                                    let json_msg = match serde_json::to_string(&ws_message) {
                                        Ok(json) => json,
                                        Err(e) => {
                                            error!("Failed to serialize events response: {}", e);
                                            let error_msg = WebSocketMessage::Error {
                                                message: format!("Serialization error: {}", e),
                                            };
                                            let error_json = serde_json::to_string(&error_msg)
                                                .unwrap_or_else(|_| "{}".to_string());
                                            let _ = sender.send(Message::Text(error_json)).await;
                                            break;
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
                                error!("gRPC error: {}", e);
                                let error_msg = WebSocketMessage::Error {
                                    message: format!("gRPC error: {}", e),
                                };
                                let error_json = serde_json::to_string(&error_msg)
                                    .unwrap_or_else(|_| "{}".to_string());
                                let _ = sender.send(Message::Text(error_json)).await;
                            },
                        }
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
