// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! WebSocket support for the v2 API.
//!
//! Provides real-time push notifications via `/v2/ws`. Clients can subscribe to:
//! - **new_blocks**: Notifications when new blocks are committed
//! - **transaction_status**: Track a specific transaction by hash until committed or timeout
//! - **events**: On-chain event notifications with powerful filtering:
//!   - Filter by event type (exact match or wildcard patterns)
//!   - Filter by multiple event types (OR logic)
//!   - Filter by transaction sender address
//!   - Filter by minimum ledger version

pub mod broadcaster;
pub mod types;

use crate::v2::{
    context::V2Context,
    error::{ErrorCode, V2Error},
};
use aptos_crypto::HashValue;
use aptos_logger::debug;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::sync::{broadcast, mpsc, RwLock};
use types::{
    BlockSummary, EventData, EventFilter, SubscriptionType, TransactionStatusData,
    WsClientMessage, WsEvent, WsServerMessage,
};

/// Subscription entry: the original type and a compiled filter (for events).
struct SubscriptionEntry {
    sub_type: SubscriptionType,
    /// Compiled event filter (only populated for `Events` subscriptions).
    event_filter: Option<EventFilter>,
}

/// GET /v2/ws -- WebSocket upgrade endpoint.
pub async fn ws_handler(
    State(ctx): State<V2Context>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, V2Error> {
    if !ctx.v2_config.websocket_enabled {
        return Err(V2Error::forbidden(
            ErrorCode::WebSocketDisabled,
            "WebSocket support is disabled on this node".to_string(),
        ));
    }

    let active = ctx.ws_active_connections().load(Ordering::Relaxed);
    if active >= ctx.v2_config.websocket_max_connections {
        return Err(V2Error::bad_request(
            ErrorCode::RateLimited,
            "WebSocket connection limit reached".to_string(),
        ));
    }

    Ok(ws.on_upgrade(move |socket| handle_ws_connection(ctx, socket)))
}

/// Handle a single WebSocket connection.
async fn handle_ws_connection(ctx: V2Context, socket: WebSocket) {
    ctx.ws_active_connections().fetch_add(1, Ordering::Relaxed);

    let (ws_sender, mut ws_receiver) = socket.split();

    // Per-connection subscription state (shared between read loop and broadcast filter).
    let subscriptions: Arc<RwLock<HashMap<String, SubscriptionEntry>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let sub_counter = Arc::new(AtomicUsize::new(0));

    // Channel for outgoing messages (fed by both read-loop responses and broadcast matches).
    let (out_tx, mut out_rx) = mpsc::channel::<WsServerMessage>(256);

    // ---- Write loop: drain out_rx → WebSocket ----
    let ws_sender = Arc::new(tokio::sync::Mutex::new(ws_sender));
    let ws_sender_clone = ws_sender.clone();
    let write_handle = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            let text = match serde_json::to_string(&msg) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let mut sender = ws_sender_clone.lock().await;
            if sender.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    // ---- Broadcast filter loop: receive WsEvent, filter by subscriptions, send matches ----
    let broadcast_rx = ctx.ws_subscribe();
    let subs_for_broadcast = subscriptions.clone();
    let out_tx_broadcast = out_tx.clone();
    let broadcast_handle = tokio::spawn(async move {
        run_broadcast_filter(broadcast_rx, subs_for_broadcast, out_tx_broadcast).await;
    });

    // ---- Read loop: process client messages ----
    let max_subs = ctx.v2_config.websocket_max_subscriptions_per_conn;
    while let Some(Ok(msg)) = ws_receiver.next().await {
        match msg {
            Message::Text(text) => {
                handle_text_message(
                    &ctx,
                    &text,
                    &subscriptions,
                    &sub_counter,
                    max_subs,
                    &out_tx,
                )
                .await;
            },
            Message::Close(_) => break,
            Message::Ping(data) => {
                // Axum handles pong automatically, but just in case:
                let mut sender = ws_sender.lock().await;
                let _ = sender.send(Message::Pong(data)).await;
            },
            _ => {}, // Ignore binary, pong
        }
    }

    // ---- Cleanup ----
    broadcast_handle.abort();
    write_handle.abort();

    ctx.ws_active_connections().fetch_sub(1, Ordering::Relaxed);
    debug!("WebSocket connection closed");
}

/// Process a single text message from the client.
async fn handle_text_message(
    ctx: &V2Context,
    text: &str,
    subscriptions: &Arc<RwLock<HashMap<String, SubscriptionEntry>>>,
    sub_counter: &Arc<AtomicUsize>,
    max_subs: usize,
    out_tx: &mpsc::Sender<WsServerMessage>,
) {
    match serde_json::from_str::<WsClientMessage>(text) {
        Ok(WsClientMessage::Subscribe { id, subscription }) => {
            let mut subs = subscriptions.write().await;

            if subs.len() >= max_subs {
                let _ = out_tx
                    .send(WsServerMessage::Error {
                        code: "SUBSCRIPTION_LIMIT".to_string(),
                        message: "Maximum subscriptions per connection reached".to_string(),
                        subscription_id: None,
                    })
                    .await;
                return;
            }

            let sub_id = id.unwrap_or_else(|| {
                let n = sub_counter.fetch_add(1, Ordering::Relaxed) + 1;
                format!("sub_{}", n)
            });

            // For transaction_status, spawn a dedicated poller task.
            if let SubscriptionType::TransactionStatus { ref hash } = subscription {
                if let Ok(hash_value) = hash
                    .strip_prefix("0x")
                    .unwrap_or(hash)
                    .parse::<HashValue>()
                {
                    let tx_ctx = ctx.clone();
                    let tx_out = out_tx.clone();
                    let tx_sub_id = sub_id.clone();
                    tokio::spawn(async move {
                        spawn_tx_status_tracker(tx_ctx, hash_value, tx_sub_id, tx_out).await;
                    });
                } else {
                    let _ = out_tx
                        .send(WsServerMessage::Error {
                            code: "INVALID_HASH".to_string(),
                            message: format!("Invalid transaction hash: {}", hash),
                            subscription_id: Some(sub_id),
                        })
                        .await;
                    return;
                }
            }

            // Build compiled event filter for Events subscriptions.
            let event_filter = if let SubscriptionType::Events {
                ref event_type,
                ref event_types,
                ref sender,
                ref start_version,
            } = subscription
            {
                Some(EventFilter::from_subscription(
                    event_type,
                    event_types,
                    sender,
                    start_version,
                ))
            } else {
                None
            };

            subs.insert(
                sub_id.clone(),
                SubscriptionEntry {
                    sub_type: subscription,
                    event_filter,
                },
            );
            let _ = out_tx
                .send(WsServerMessage::Subscribed { id: sub_id })
                .await;
        },
        Ok(WsClientMessage::Unsubscribe { id }) => {
            let mut subs = subscriptions.write().await;
            if subs.remove(&id).is_some() {
                let _ = out_tx
                    .send(WsServerMessage::Unsubscribed { id })
                    .await;
            } else {
                let _ = out_tx
                    .send(WsServerMessage::Error {
                        code: "UNKNOWN_SUBSCRIPTION".to_string(),
                        message: format!("No subscription with id: {}", id),
                        subscription_id: Some(id),
                    })
                    .await;
            }
        },
        Ok(WsClientMessage::Ping { nonce }) => {
            let _ = out_tx.send(WsServerMessage::Pong { nonce }).await;
        },
        Err(e) => {
            let _ = out_tx
                .send(WsServerMessage::Error {
                    code: "INVALID_MESSAGE".to_string(),
                    message: format!("Failed to parse message: {}", e),
                    subscription_id: None,
                })
                .await;
        },
    }
}

/// Receive broadcast events, filter by active subscriptions, forward matches.
async fn run_broadcast_filter(
    mut rx: broadcast::Receiver<WsEvent>,
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionEntry>>>,
    out_tx: mpsc::Sender<WsServerMessage>,
) {
    loop {
        match rx.recv().await {
            Ok(event) => {
                let subs = subscriptions.read().await;
                for (id, entry) in subs.iter() {
                    let messages = match_event(&event, id, entry);
                    for msg in messages {
                        if out_tx.send(msg).await.is_err() {
                            return; // Connection closed
                        }
                    }
                }
            },
            Err(broadcast::error::RecvError::Lagged(n)) => {
                let _ = out_tx
                    .send(WsServerMessage::Error {
                        code: "LAGGED".to_string(),
                        message: format!("Missed {} events due to slow consumption", n),
                        subscription_id: None,
                    })
                    .await;
            },
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}

/// Determine if a broadcast event matches a subscription. Returns zero or more
/// messages (one per matching event within a single `WsEvent::Events` broadcast).
fn match_event(
    event: &WsEvent,
    subscription_id: &str,
    entry: &SubscriptionEntry,
) -> Vec<WsServerMessage> {
    match (event, &entry.sub_type) {
        (
            WsEvent::NewBlock {
                height,
                hash,
                timestamp_usec,
                first_version,
                last_version,
                num_transactions,
            },
            SubscriptionType::NewBlocks,
        ) => vec![WsServerMessage::NewBlock {
            subscription_id: subscription_id.to_string(),
            data: BlockSummary {
                height: *height,
                hash: hash.clone(),
                timestamp_usec: *timestamp_usec,
                first_version: *first_version,
                last_version: *last_version,
                num_transactions: *num_transactions,
            },
        }],

        (
            WsEvent::Events {
                version,
                sender,
                events,
            },
            SubscriptionType::Events { .. },
        ) => {
            let filter = entry
                .event_filter
                .as_ref()
                .expect("EventFilter must be set for Events subscription");

            // Apply version and sender filters first (they apply to the whole txn).
            if !filter.matches_version(*version) {
                return vec![];
            }
            if !filter.matches_sender(sender) {
                return vec![];
            }

            // Filter individual events by type and emit one message per match.
            events
                .iter()
                .filter(|(_, etype, _)| filter.matches_type(etype))
                .map(|(index, etype, data)| WsServerMessage::Event {
                    subscription_id: subscription_id.to_string(),
                    data: EventData {
                        version: *version,
                        event_index: *index,
                        event_type: etype.clone(),
                        data: data.clone(),
                        sender: sender.clone(),
                    },
                })
                .collect()
        },

        // TransactionStatus subscriptions are handled by dedicated poller tasks,
        // not through the broadcast channel.
        _ => vec![],
    }
}

/// Poll the DB for a specific transaction until committed or timeout.
async fn spawn_tx_status_tracker(
    ctx: V2Context,
    hash: HashValue,
    subscription_id: String,
    tx: mpsc::Sender<WsServerMessage>,
) {
    let timeout = Duration::from_millis(ctx.v2_config.wait_by_hash_timeout_ms);
    let poll_interval = Duration::from_millis(ctx.v2_config.wait_by_hash_poll_interval_ms);
    let deadline = Instant::now() + timeout;

    // Send initial "pending" status.
    let _ = tx
        .send(WsServerMessage::TransactionStatusUpdate {
            subscription_id: subscription_id.clone(),
            data: TransactionStatusData::Pending {
                hash: format!("0x{}", hash),
            },
        })
        .await;

    loop {
        if Instant::now() >= deadline {
            let _ = tx
                .send(WsServerMessage::TransactionStatusUpdate {
                    subscription_id: subscription_id.clone(),
                    data: TransactionStatusData::NotFound {
                        hash: format!("0x{}", hash),
                    },
                })
                .await;
            break;
        }

        if let Ok(ledger_info) = ctx.ledger_info() {
            if let Ok(Some(txn)) = ctx
                .inner()
                .get_transaction_by_hash(hash, ledger_info.version())
            {
                let success = txn.info.status().is_success();
                let vm_status = format!("{:?}", txn.info.status());
                let _ = tx
                    .send(WsServerMessage::TransactionStatusUpdate {
                        subscription_id: subscription_id.clone(),
                        data: TransactionStatusData::Committed {
                            hash: format!("0x{}", hash),
                            version: txn.version,
                            success,
                            vm_status,
                        },
                    })
                    .await;
                break;
            }
        }

        tokio::time::sleep(poll_interval).await;
    }
}
