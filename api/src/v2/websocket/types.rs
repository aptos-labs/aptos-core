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
        /// Single event type filter (backward compatibility).
        /// Supports exact match or wildcard patterns:
        ///   - `"0x1::coin::DepositEvent"` — exact match
        ///   - `"0x1::coin::*"` — all events from module `0x1::coin`
        ///   - `"0x1::*"` — all events from address `0x1`
        /// If both `event_type` and `event_types` are set, they are merged (OR logic).
        event_type: Option<String>,
        /// Multiple event type filters (OR logic). Each entry supports the same
        /// exact and wildcard patterns as `event_type`.
        /// When absent, `event_type` is used. When both are absent, matches all events.
        event_types: Option<Vec<String>>,
        /// Filter by transaction sender address (hex, with or without `0x` prefix).
        /// Only events from transactions sent by this address are delivered.
        sender: Option<String>,
        /// Only deliver events at or after this ledger version.
        start_version: Option<u64>,
    },
}

// ---- Event filter logic ----

/// Compiled event filter created from `SubscriptionType::Events` fields.
/// This avoids re-parsing filter strings on every broadcast event.
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// Compiled type patterns (empty = match all types).
    pub type_patterns: Vec<EventTypePattern>,
    /// Normalized sender address filter (lowercase, no `0x` prefix).
    pub sender: Option<String>,
    /// Minimum version (inclusive).
    pub start_version: Option<u64>,
}

/// A single event-type match pattern.
#[derive(Debug, Clone)]
pub enum EventTypePattern {
    /// Exact match: `"0x1::coin::DepositEvent"`.
    Exact(String),
    /// Prefix match: `"0x1::coin::*"` matches any type starting with `"0x1::coin::"`.
    Prefix(String),
}

impl EventFilter {
    /// Build a filter from the subscription fields.
    pub fn from_subscription(
        event_type: &Option<String>,
        event_types: &Option<Vec<String>>,
        sender: &Option<String>,
        start_version: &Option<u64>,
    ) -> Self {
        let mut raw_patterns: Vec<String> = Vec::new();

        if let Some(et) = event_type {
            raw_patterns.push(et.clone());
        }
        if let Some(ets) = event_types {
            raw_patterns.extend(ets.iter().cloned());
        }

        let type_patterns = raw_patterns
            .into_iter()
            .map(|p| {
                if let Some(prefix) = p.strip_suffix('*') {
                    EventTypePattern::Prefix(prefix.to_string())
                } else {
                    EventTypePattern::Exact(p)
                }
            })
            .collect();

        let sender = sender.as_ref().map(|s| {
            s.strip_prefix("0x")
                .unwrap_or(s)
                .to_lowercase()
        });

        Self {
            type_patterns,
            sender,
            start_version: *start_version,
        }
    }

    /// Check if a specific event type string matches this filter's type patterns.
    /// Returns true if there are no type patterns (match all) or any pattern matches.
    pub fn matches_type(&self, event_type: &str) -> bool {
        if self.type_patterns.is_empty() {
            return true;
        }
        self.type_patterns.iter().any(|p| match p {
            EventTypePattern::Exact(exact) => event_type == exact,
            EventTypePattern::Prefix(prefix) => event_type.starts_with(prefix),
        })
    }

    /// Check if a sender address matches the filter.
    /// Returns true if no sender filter is set.
    pub fn matches_sender(&self, sender: &Option<String>) -> bool {
        match (&self.sender, sender) {
            (Some(filter_sender), Some(event_sender)) => {
                let normalized = event_sender
                    .strip_prefix("0x")
                    .unwrap_or(event_sender)
                    .to_lowercase();
                *filter_sender == normalized
            },
            (Some(_), None) => false, // Filter is set but event has no sender
            (None, _) => true,        // No filter
        }
    }

    /// Check if a version is at or after the start_version filter.
    /// Returns true if no start_version filter is set.
    pub fn matches_version(&self, version: u64) -> bool {
        match self.start_version {
            Some(sv) => version >= sv,
            None => true,
        }
    }

    /// Check if a complete event matches all filter criteria.
    pub fn matches(&self, event_type: &str, sender: &Option<String>, version: u64) -> bool {
        self.matches_version(version)
            && self.matches_sender(sender)
            && self.matches_type(event_type)
    }
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
    /// Address of the transaction sender (if from a user transaction).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
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
        /// Address of the transaction sender (None for non-user txns).
        sender: Option<String>,
        /// (event_index, event_type, event_data_json)
        events: Vec<(u64, String, serde_json::Value)>,
    },
}

// ---- Unit tests for EventFilter ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let filter = EventFilter::from_subscription(
            &Some("0x1::coin::DepositEvent".to_string()),
            &None,
            &None,
            &None,
        );
        assert!(filter.matches_type("0x1::coin::DepositEvent"));
        assert!(!filter.matches_type("0x1::coin::WithdrawEvent"));
        assert!(!filter.matches_type("0x1::aptos_coin::MintEvent"));
    }

    #[test]
    fn test_module_wildcard() {
        let filter = EventFilter::from_subscription(
            &Some("0x1::coin::*".to_string()),
            &None,
            &None,
            &None,
        );
        assert!(filter.matches_type("0x1::coin::DepositEvent"));
        assert!(filter.matches_type("0x1::coin::WithdrawEvent"));
        assert!(!filter.matches_type("0x1::aptos_coin::MintEvent"));
    }

    #[test]
    fn test_address_wildcard() {
        let filter = EventFilter::from_subscription(
            &Some("0x1::*".to_string()),
            &None,
            &None,
            &None,
        );
        assert!(filter.matches_type("0x1::coin::DepositEvent"));
        assert!(filter.matches_type("0x1::aptos_coin::MintEvent"));
        assert!(!filter.matches_type("0x2::nft::TransferEvent"));
    }

    #[test]
    fn test_multiple_type_filters_or_logic() {
        let filter = EventFilter::from_subscription(
            &None,
            &Some(vec![
                "0x1::coin::DepositEvent".to_string(),
                "0x1::coin::WithdrawEvent".to_string(),
            ]),
            &None,
            &None,
        );
        assert!(filter.matches_type("0x1::coin::DepositEvent"));
        assert!(filter.matches_type("0x1::coin::WithdrawEvent"));
        assert!(!filter.matches_type("0x1::coin::MintEvent"));
    }

    #[test]
    fn test_merge_event_type_and_event_types() {
        let filter = EventFilter::from_subscription(
            &Some("0x1::coin::DepositEvent".to_string()),
            &Some(vec!["0x2::nft::TransferEvent".to_string()]),
            &None,
            &None,
        );
        assert!(filter.matches_type("0x1::coin::DepositEvent"));
        assert!(filter.matches_type("0x2::nft::TransferEvent"));
        assert!(!filter.matches_type("0x1::coin::WithdrawEvent"));
    }

    #[test]
    fn test_no_filters_matches_all() {
        let filter = EventFilter::from_subscription(&None, &None, &None, &None);
        assert!(filter.matches_type("anything::at::all"));
    }

    #[test]
    fn test_sender_filter() {
        let filter = EventFilter::from_subscription(
            &None,
            &None,
            &Some("0xABCD".to_string()),
            &None,
        );
        assert!(filter.matches_sender(&Some("0xabcd".to_string())));
        assert!(filter.matches_sender(&Some("abcd".to_string())));
        assert!(!filter.matches_sender(&Some("0x1234".to_string())));
        assert!(!filter.matches_sender(&None));
    }

    #[test]
    fn test_no_sender_filter_matches_all() {
        let filter = EventFilter::from_subscription(&None, &None, &None, &None);
        assert!(filter.matches_sender(&Some("0x1234".to_string())));
        assert!(filter.matches_sender(&None));
    }

    #[test]
    fn test_version_filter() {
        let filter = EventFilter::from_subscription(&None, &None, &None, &Some(100));
        assert!(!filter.matches_version(50));
        assert!(!filter.matches_version(99));
        assert!(filter.matches_version(100));
        assert!(filter.matches_version(200));
    }

    #[test]
    fn test_no_version_filter() {
        let filter = EventFilter::from_subscription(&None, &None, &None, &None);
        assert!(filter.matches_version(0));
        assert!(filter.matches_version(999999));
    }

    #[test]
    fn test_combined_filter() {
        let filter = EventFilter::from_subscription(
            &Some("0x1::coin::*".to_string()),
            &None,
            &Some("0xABC".to_string()),
            &Some(50),
        );
        // Matches: right type, right sender, right version
        assert!(filter.matches(
            "0x1::coin::DepositEvent",
            &Some("0xabc".to_string()),
            100
        ));
        // Wrong type
        assert!(!filter.matches(
            "0x2::nft::Transfer",
            &Some("0xabc".to_string()),
            100
        ));
        // Wrong sender
        assert!(!filter.matches(
            "0x1::coin::DepositEvent",
            &Some("0x999".to_string()),
            100
        ));
        // Version too low
        assert!(!filter.matches(
            "0x1::coin::DepositEvent",
            &Some("0xabc".to_string()),
            10
        ));
    }
}
