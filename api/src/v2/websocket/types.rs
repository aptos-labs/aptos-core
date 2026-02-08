// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! WebSocket message types for the v2 API.

use serde::{Deserialize, Serialize};

// ---- Client → Server ----

/// Messages sent by the client over the WebSocket connection.
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum WsClientMessage {
    /// Subscribe to a stream of events.
    Subscribe {
        /// Client-provided subscription ID (server generates if absent).
        id: Option<String>,
        /// Type of subscription.
        #[serde(flatten)]
        subscription: SubscriptionType,
    },
    /// Unsubscribe from a previously created subscription.
    Unsubscribe {
        /// The subscription ID to remove.
        id: String,
    },
    /// Ping for keepalive.
    Ping {
        /// Opaque nonce echoed back in pong.
        nonce: Option<u64>,
    },
}

/// The type of subscription a client can create.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SubscriptionType {
    /// Subscribe to new committed blocks.
    NewBlocks,
    /// Subscribe to transaction status updates for a specific hash.
    TransactionStatus {
        hash: String,
    },
    /// Subscribe to events matching a filter.
    Events {
        /// Event type to filter (e.g., "0x1::coin::DepositEvent").
        /// If None, receives all events.
        event_type: Option<String>,
        /// Account address to filter events for.
        account: Option<String>,
    },
}

// ---- Server → Client ----

/// Messages sent by the server to the client.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsServerMessage {
    /// Acknowledgement of a successful subscription.
    Subscribed { id: String },
    /// Acknowledgement of an unsubscription.
    Unsubscribed { id: String },
    /// Pong response to a ping.
    Pong { nonce: Option<u64> },
    /// New block notification.
    NewBlock {
        subscription_id: String,
        data: BlockSummary,
    },
    /// Transaction status update.
    TransactionStatusUpdate {
        subscription_id: String,
        data: TransactionStatusData,
    },
    /// Event notification.
    Event {
        subscription_id: String,
        data: EventData,
    },
    /// Error message.
    Error {
        code: String,
        message: String,
        /// If related to a specific subscription.
        subscription_id: Option<String>,
    },
}

/// Summary of a committed block.
#[derive(Debug, Clone, Serialize)]
pub struct BlockSummary {
    pub height: u64,
    pub hash: String,
    pub timestamp_usec: u64,
    pub first_version: u64,
    pub last_version: u64,
    pub num_transactions: u64,
}

/// Transaction status update data.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TransactionStatusData {
    /// Transaction is in the mempool, pending execution.
    Pending { hash: String },
    /// Transaction has been committed on-chain.
    Committed {
        hash: String,
        version: u64,
        success: bool,
        vm_status: String,
    },
    /// Transaction was not found after timeout (dropped or expired).
    NotFound { hash: String },
}

/// An on-chain event notification.
#[derive(Debug, Clone, Serialize)]
pub struct EventData {
    pub version: u64,
    pub event_index: u64,
    pub event_type: String,
    pub data: serde_json::Value,
}

// ---- Internal broadcast types ----

/// Internal event type broadcast from the block poller to all connections.
/// Each connection filters these against its active subscriptions.
#[derive(Clone, Debug)]
pub enum WsEvent {
    NewBlock {
        height: u64,
        hash: String,
        timestamp_usec: u64,
        first_version: u64,
        last_version: u64,
        num_transactions: u64,
    },
    /// A batch of events from a committed transaction.
    Events {
        version: u64,
        events: Vec<(u64, String, serde_json::Value)>, // (index, type, data)
    },
}
