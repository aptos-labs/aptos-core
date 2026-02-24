// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Server-Sent Events (SSE) endpoints for the v2 API.
//!
//! SSE provides a lighter-weight alternative to WebSocket for one-way
//! server-to-client streaming over standard HTTP. Clients use the
//! browser-native `EventSource` API or any HTTP client that supports
//! streaming responses.
//!
//! Endpoints:
//! - `GET /v2/sse/blocks` — stream of new committed block summaries
//! - `GET /v2/sse/events` — stream of filtered on-chain events
//!
//! Both endpoints reuse the same broadcast channel infrastructure as
//! WebSocket, so they share the background block poller.

use crate::v2::{
    context::V2Context,
    error::{ErrorCode, V2Error},
    websocket::types::{EventFilter, WsEvent},
};
use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
};
use futures::stream::{self, Stream};
use serde::Deserialize;
use std::{convert::Infallible, time::Duration};
use tokio::sync::broadcast;
use utoipa::IntoParams;

/// Query parameters for the SSE blocks endpoint.
#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct SseBlocksParams {
    /// Only emit blocks with height strictly greater than this value.
    /// Useful for resuming after a reconnect: set to the last received
    /// block height.
    pub after_height: Option<u64>,
}

/// Query parameters for the SSE events endpoint.
#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct SseEventsParams {
    /// Comma-separated list of event type patterns. Each entry can be:
    /// - Exact: `0x1::coin::DepositEvent`
    /// - Module wildcard: `0x1::coin::*`
    /// - Address wildcard: `0x1::*`
    /// Multiple patterns use OR logic. Omit to match all event types.
    pub event_types: Option<String>,
    /// Filter by transaction sender address (hex, case-insensitive).
    /// Only events from transactions sent by this address are delivered.
    pub sender: Option<String>,
    /// Only deliver events at or after this ledger version (inclusive).
    pub start_version: Option<u64>,
}

/// GET /v2/sse/blocks — SSE stream of new block notifications.
///
/// Each SSE event has:
/// - `event: block`
/// - `id: <block_height>` (tracks Last-Event-ID for reconnection)
/// - `data: { height, hash, timestamp_usec, first_version, last_version, num_transactions }`
///
/// A `lagged` event is emitted if the client falls behind the broadcast buffer.
#[utoipa::path(
    get,
    path = "/v2/sse/blocks",
    tag = "SSE",
    params(SseBlocksParams),
    responses(
        (status = 200, description = "SSE stream of new blocks", content_type = "text/event-stream"),
        (status = 400, description = "SSE disabled", body = V2Error),
    )
)]
pub async fn sse_blocks_handler(
    State(ctx): State<V2Context>,
    Query(params): Query<SseBlocksParams>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, V2Error> {
    if !ctx.v2_config.sse_enabled {
        return Err(V2Error::bad_request(
            ErrorCode::ServiceUnavailable,
            "SSE is disabled on this node",
        ));
    }

    let rx = ctx.ws_subscribe();
    let shutdown_rx = ctx.shutdown_receiver();
    let after_height = params.after_height;

    // Channel to decouple the broadcast consumer from the SSE stream.
    let (tx, mpsc_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(256);

    // Background task: read from broadcast, filter, and forward to the mpsc channel.
    // Note: we avoid `tokio::select!` with `broadcast_rx.recv()` because the
    // `select!` output type includes `*mut ()` from the broadcast internals,
    // making the future `!Send`. Instead we check the shutdown flag after each recv.
    tokio::spawn(async move {
        let mut broadcast_rx = rx;
        loop {
            // Check shutdown before blocking on recv.
            if *shutdown_rx.borrow() {
                break;
            }

            match broadcast_rx.recv().await {
                Ok(WsEvent::NewBlock {
                    height,
                    hash,
                    timestamp_usec,
                    first_version,
                    last_version,
                    num_transactions,
                }) => {
                    if let Some(after) = after_height {
                        if height <= after {
                            continue;
                        }
                    }

                    let data = serde_json::json!({
                        "height": height,
                        "hash": hash,
                        "timestamp_usec": timestamp_usec,
                        "first_version": first_version,
                        "last_version": last_version,
                        "num_transactions": num_transactions,
                    });

                    let sse_event = Event::default()
                        .event("block")
                        .id(height.to_string())
                        .json_data(data)
                        .unwrap_or_else(|_| Event::default().data("serialization_error"));

                    if tx.send(Ok(sse_event)).await.is_err() {
                        break; // Client disconnected
                    }
                },
                Ok(_) => continue, // Skip non-block events
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    let event = Event::default()
                        .event("lagged")
                        .data(format!("Skipped {} events (slow consumer)", n));
                    if tx.send(Ok(event)).await.is_err() {
                        break;
                    }
                },
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // Convert mpsc::Receiver into a Stream.
    let stream = stream::unfold(mpsc_rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    });

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

/// GET /v2/sse/events — SSE stream of filtered on-chain events.
///
/// Query parameters control which events are streamed. The same powerful
/// filtering available in WebSocket subscriptions is supported here:
/// exact match, module/address wildcards, sender filtering, and version floor.
///
/// Each SSE event has:
/// - `event: event`
/// - `id: <version>` (tracks Last-Event-ID for reconnection)
/// - `data: { version, sender, event_type, sequence_number, data }`
///
/// One SSE event is emitted per matching on-chain event (a single transaction
/// may produce multiple SSE events if it emits multiple matching events).
#[utoipa::path(
    get,
    path = "/v2/sse/events",
    tag = "SSE",
    params(SseEventsParams),
    responses(
        (status = 200, description = "SSE stream of filtered events", content_type = "text/event-stream"),
        (status = 400, description = "SSE disabled", body = V2Error),
    )
)]
pub async fn sse_events_handler(
    State(ctx): State<V2Context>,
    Query(params): Query<SseEventsParams>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, V2Error> {
    if !ctx.v2_config.sse_enabled {
        return Err(V2Error::bad_request(
            ErrorCode::ServiceUnavailable,
            "SSE is disabled on this node",
        ));
    }

    // Parse comma-separated event type patterns into individual strings.
    let event_types: Vec<String> = params
        .event_types
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // Build a compiled EventFilter for efficient per-event matching.
    // We use `from_subscription` with `event_type=None` since we already
    // merged all patterns into the `event_types` vec.
    let filter = EventFilter::from_subscription(&None, &Some(event_types), &params.sender, &params.start_version);

    let rx = ctx.ws_subscribe();
    let shutdown_rx = ctx.shutdown_receiver();

    let (tx, mpsc_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(256);

    // Background task: read from broadcast, apply filter, forward matching events.
    // Uses the same `borrow()` shutdown check as the blocks handler (see note above).
    tokio::spawn(async move {
        let mut broadcast_rx = rx;
        loop {
            if *shutdown_rx.borrow() {
                return;
            }

            match broadcast_rx.recv().await {
                Ok(WsEvent::Events {
                    version,
                    sender,
                    events,
                }) => {
                    if !filter.matches_version(version) {
                        continue;
                    }
                    if !filter.matches_sender(&sender) {
                        continue;
                    }

                    // Emit one SSE event per matching on-chain event.
                    for (seq, event_type, data) in &events {
                        if !filter.matches_type(event_type) {
                            continue;
                        }

                        let event_data = serde_json::json!({
                            "version": version,
                            "sender": sender,
                            "event_type": event_type,
                            "sequence_number": seq,
                            "data": data,
                        });

                        let sse_event = Event::default()
                            .event("event")
                            .id(version.to_string())
                            .json_data(event_data)
                            .unwrap_or_else(|_| Event::default().data("serialization_error"));

                        if tx.send(Ok(sse_event)).await.is_err() {
                            return; // Client disconnected
                        }
                    }
                },
                Ok(_) => continue, // Skip non-event messages
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    let event = Event::default()
                        .event("lagged")
                        .data(format!("Skipped {} events (slow consumer)", n));
                    if tx.send(Ok(event)).await.is_err() {
                        return;
                    }
                },
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    });

    let stream = stream::unfold(mpsc_rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    });

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}
