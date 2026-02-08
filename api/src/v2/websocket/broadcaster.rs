// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Background block poller that broadcasts new blocks and events to
//! all connected WebSocket clients via a `tokio::sync::broadcast` channel.

use super::types::WsEvent;
use crate::v2::context::V2Context;
use aptos_logger::{debug, warn};
use aptos_types::contract_event::ContractEvent;
use std::time::Duration;
use tokio::sync::broadcast;

/// Capacity of the broadcast channel. Slow consumers that fall behind
/// this many messages will receive a `RecvError::Lagged`.
pub const BROADCAST_CHANNEL_CAPACITY: usize = 4096;

/// Create the broadcast channel for WebSocket events.
pub fn create_broadcast_channel() -> (broadcast::Sender<WsEvent>, broadcast::Receiver<WsEvent>) {
    broadcast::channel(BROADCAST_CHANNEL_CAPACITY)
}

/// Background task that polls the DB for new committed blocks and
/// broadcasts events to all connected WebSocket clients.
///
/// This task runs for the lifetime of the v2 API server.
pub async fn run_block_poller(ctx: V2Context, ws_tx: broadcast::Sender<WsEvent>) {
    let mut last_known_height: Option<u64> = None;
    let mut interval = tokio::time::interval(Duration::from_millis(100));

    loop {
        interval.tick().await;

        // Skip if nobody is listening.
        if ws_tx.receiver_count() == 0 {
            continue;
        }

        let ledger_info = match ctx.ledger_info() {
            Ok(info) => info,
            Err(_) => continue,
        };

        let current_height: u64 = ledger_info.block_height.into();

        let start_height = match last_known_height {
            Some(h) if h >= current_height => continue,
            Some(h) => h + 1,
            None => current_height, // First iteration: only emit current block
        };

        // Emit block events for each new block since we last checked.
        for height in start_height..=current_height {
            match ctx.inner().db.get_block_info_by_height(height) {
                Ok((first_version, last_version, block_event)) => {
                    let hash = block_event
                        .hash()
                        .map(|h| h.to_hex_literal())
                        .unwrap_or_default();

                    let num_transactions = last_version - first_version + 1;

                    // Broadcast new block event.
                    let _ = ws_tx.send(WsEvent::NewBlock {
                        height,
                        hash,
                        timestamp_usec: block_event.proposed_time(),
                        first_version,
                        last_version,
                        num_transactions,
                    });

                    // Also broadcast events from this block's transactions.
                    emit_block_events(&ctx, first_version, last_version, ledger_info.version(), &ws_tx);
                },
                Err(e) => {
                    debug!("Block {} not found during WS poll: {}", height, e);
                    continue;
                },
            }
        }

        last_known_height = Some(current_height);
    }
}

/// Read transactions in a block and emit their events.
fn emit_block_events(
    ctx: &V2Context,
    first_version: u64,
    last_version: u64,
    ledger_version: u64,
    ws_tx: &broadcast::Sender<WsEvent>,
) {
    let count = (last_version - first_version + 1).min(u16::MAX as u64) as u16;
    let txns = match ctx
        .inner()
        .get_transactions(first_version, count, ledger_version)
    {
        Ok(txns) => txns,
        Err(e) => {
            warn!("Failed to read txns for WS event broadcast: {}", e);
            return;
        },
    };

    for txn in txns {
        let events: Vec<(u64, String, serde_json::Value)> = txn
            .events
            .iter()
            .enumerate()
            .map(|(idx, event)| {
                let event_type = match event {
                    ContractEvent::V1(v1) => v1.type_tag().to_canonical_string(),
                    ContractEvent::V2(v2) => v2.type_tag().to_canonical_string(),
                };
                // Event data is BCS-encoded; for now we send the type string and null data.
                // Full JSON conversion requires the MoveConverter which needs a state view.
                // TODO: Optionally convert event data to JSON using MoveConverter.
                (idx as u64, event_type, serde_json::Value::Null)
            })
            .collect();

        if !events.is_empty() {
            let _ = ws_tx.send(WsEvent::Events {
                version: txn.version,
                events,
            });
        }
    }
}
