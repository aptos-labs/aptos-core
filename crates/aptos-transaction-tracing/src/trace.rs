// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_config::{config::TransactionTracingConfig, network_id::PeerNetworkId};
use aptos_logger::{error, warn};
use chrono::{DateTime, Utc};
use move_core_types::vm_status::DiscardedVMStatus;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc, time::SystemTime};

// Useful strings for compact trace logging
const COMMA_DELIMITER_STRING: &str = ", ";
const FORWARDED_TO_QUORUM_STORE_STRING: &str = "QS";
const MISSING_TRACE_ENTRY_STRING: &str = "?";
const TRANSACTION_COMMITTED_STRING: &str = "C";
const TRANSACTION_EXPIRED_STRING: &str = "E";
const TRANSACTION_PENDING_STRING: &str = "P";
const TRANSACTION_PULLED_BY_QUORUM_STORE_STRING: &str = "QS-P";
const TRANSACTION_REJECTED_BY_QUORUM_STORE_STRING: &str = "QS-R";

/// A transaction trace, made up of multiple partial traces
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransactionTrace {
    // The configuration for transaction tracing
    transaction_tracing_config: Arc<TransactionTracingConfig>,

    // The partial trace authored by this node
    authored_partial_trace: PartialTrace,

    // A collection of partial traces from our peers (indexed by peer)
    peer_partial_traces: BTreeMap<PeerNetworkId, PartialTrace>,
}

impl TransactionTrace {
    pub fn new(
        transaction_tracing_config: Arc<TransactionTracingConfig>,
        authored_partial_trace: PartialTrace,
    ) -> Self {
        Self {
            transaction_tracing_config,
            authored_partial_trace,
            peer_partial_traces: BTreeMap::new(),
        }
    }

    /// Adds the given peer partial trace to this transaction trace. If the maximum
    /// number of partial traces has already been reached, the new trace is dropped.
    fn add_peer_partial_trace(
        &mut self,
        peer_network_id: PeerNetworkId,
        peer_partial_trace: PartialTrace,
    ) {
        // Get the maximum number of partial peer traces allowed
        let max_num_partial_peer_traces = self
            .transaction_tracing_config
            .max_num_partial_peer_traces_per_transaction;

        // Add the peer partial trace if we have capacity
        if !self.peer_partial_traces.contains_key(&peer_network_id)
            && self.peer_partial_traces.len() >= max_num_partial_peer_traces
        {
            warn!(
                "Cannot add new peer partial trace from {}. Maximum number of traces ({}) reached.",
                peer_network_id, max_num_partial_peer_traces
            );
        } else {
            self.peer_partial_traces
                .insert(peer_network_id, peer_partial_trace);
        }
    }

    /// Returns true iff the trace is complete. Traces are considered
    /// complete if the corresponding transaction was committed,
    /// and enough time has elapsed for partial traces to be collected.
    pub fn is_complete(&self) -> bool {
        // If the transaction was locally committed, check if enough time has elapsed
        let max_time_to_collect_partial_traces_secs = self
            .transaction_tracing_config
            .max_time_to_collect_partial_traces_secs;
        if let Some(commit_timestamp) = self.local_commit_timestamp() {
            if let Some(elapsed_since_commit_secs) = duration_since_seconds(commit_timestamp) {
                return elapsed_since_commit_secs >= max_time_to_collect_partial_traces_secs;
            }
        }

        false
    }

    /// Returns true iff the trace is expired. Traces are considered
    /// expired if the corresponding transaction expired, and enough
    /// time has elapsed for all partial traces to be collected.
    pub fn is_expired(&self) -> bool {
        // If the transaction was locally expired, check if enough time has elapsed
        let max_time_to_collect_partial_traces_secs = self
            .transaction_tracing_config
            .max_time_to_collect_partial_traces_secs;
        if let Some(expiration_timestamp) = self.local_expiration_timestamp() {
            if let Some(elapsed_since_expiration_secs) =
                duration_since_seconds(expiration_timestamp)
            {
                return elapsed_since_expiration_secs >= max_time_to_collect_partial_traces_secs;
            }
        }

        false
    }

    /// Returns true iff the trace is garbage collected. Traces are
    /// garbage collected when the corresponding transaction is pending,
    /// but the maximum allowed time for trace monitoring has occurred.
    pub fn is_garbage_collected(&self) -> bool {
        // Check if the maximum allowed time for trace monitoring has elapsed
        if let Some(pending_timestamp) = self.local_pending_timestamp() {
            if let Some(elapsed_since_pending_secs) = duration_since_seconds(pending_timestamp) {
                let max_time_to_trace_pending_transactions_secs = self
                    .transaction_tracing_config
                    .max_time_to_trace_pending_transactions_secs;
                return elapsed_since_pending_secs >= max_time_to_trace_pending_transactions_secs;
            }
        }

        false
    }

    /// Returns a reference to the authored partial trace
    pub fn get_authored_partial_trace(&self) -> &PartialTrace {
        &self.authored_partial_trace
    }

    /// Returns a compact string representation of the transaction route (for logging)
    pub fn get_transaction_route_string(
        &self,
        peer_network_id_to_int: &BTreeMap<PeerNetworkId, usize>,
    ) -> String {
        // Create an empty list of trace route strings
        let mut trace_route_strings = vec![];

        // Add the authored partial trace to the route
        let authored_partial_trace = &self.authored_partial_trace;
        let authored_peer_id = authored_partial_trace.trace_author();
        let authored_peer_string =
            get_peer_string_for_trace(authored_peer_id, peer_network_id_to_int);
        trace_route_strings.push(authored_peer_string);

        // Determine the rest of the route based on the forwarded destinations
        let mut current_partial_trace = authored_partial_trace;
        loop {
            match current_partial_trace.transaction_forwarded_to {
                Some(TransactionDestination::RemotePeer(peer_network_id)) => {
                    // Add the peer to the route
                    let peer_string =
                        get_peer_string_for_trace(&peer_network_id, peer_network_id_to_int);
                    trace_route_strings.push(peer_string);

                    // Update the current partial trace
                    match self.peer_partial_traces.get(&peer_network_id) {
                        Some(partial_trace) => current_partial_trace = partial_trace,
                        None => {
                            // Update the route with a missing entry and break (we can't continue)
                            let all_known_peer_ids: Vec<&PeerNetworkId> =
                                self.peer_partial_traces.keys().collect();
                            warn!(
                                "Missing partial trace entry for peer {} in transaction route. All known peers {:?}",
                                peer_network_id, all_known_peer_ids
                            );
                            trace_route_strings.push(MISSING_TRACE_ENTRY_STRING.to_string());
                            break; // Can't continue the route
                        },
                    }
                },
                None => {
                    // If this is the last peer in the route, we should see
                    // if the transaction was pulled by quorum store.
                    if current_partial_trace
                        .transaction_status_timeline
                        .contains_key(&TransactionStatus::PulledByQuorumStore)
                    {
                        trace_route_strings.push(FORWARDED_TO_QUORUM_STORE_STRING.into());
                    }
                    break; // End of the route
                },
            }
        }

        // Return the compact route string
        trace_route_strings.join(COMMA_DELIMITER_STRING)
    }

    /// Returns a compact string representation of the local status timeline (for logging)
    pub fn get_local_timeline_string(&self) -> String {
        get_timeline_string(&self.authored_partial_trace.transaction_status_timeline)
    }

    /// Returns a compact string representation of the peer timelines (for logging)
    pub fn get_peer_timeline_string(
        &self,
        peer_network_id_to_int: &BTreeMap<PeerNetworkId, usize>,
    ) -> String {
        let mut peer_timeline_strings = vec![];
        for (peer_network_id, partial_trace) in &self.peer_partial_traces {
            // Get the peer string
            let peer_string = get_peer_string_for_trace(peer_network_id, peer_network_id_to_int);

            // Get the timeline string
            let timeline_string = get_timeline_string(&partial_trace.transaction_status_timeline);

            // Combine the peer string and timeline string
            let peer_timeline_string = format!("[{}: {}]", peer_string, timeline_string);
            peer_timeline_strings.push(peer_timeline_string);
        }

        // Return the final string
        peer_timeline_strings.join(COMMA_DELIMITER_STRING)
    }

    /// Returns a mutable reference to the authored partial trace
    pub fn get_mut_authored_partial_trace(&mut self) -> &mut PartialTrace {
        &mut self.authored_partial_trace
    }

    /// Returns a reference to the peer partial traces
    pub fn get_peer_partial_traces(&self) -> &BTreeMap<PeerNetworkId, PartialTrace> {
        &self.peer_partial_traces
    }

    /// Returns the time when the transaction was locally committed.
    /// If the transaction was not locally committed, returns None.
    fn local_commit_timestamp(&self) -> Option<SystemTime> {
        let transaction_status_timeline = &self.authored_partial_trace.transaction_status_timeline;
        match transaction_status_timeline.get(&TransactionStatus::Committed) {
            Some(TransactionStatusMetadata::Timestamp(timestamp)) => Some(*timestamp),
            _ => None,
        }
    }

    /// Returns the time when the transaction locally expired.
    /// If the transaction was not locally expired, returns None.
    fn local_expiration_timestamp(&self) -> Option<SystemTime> {
        let transaction_status_timeline = &self.authored_partial_trace.transaction_status_timeline;
        match transaction_status_timeline.get(&TransactionStatus::Expired) {
            Some(TransactionStatusMetadata::TimestampAndExpiry(timestamp, _)) => Some(*timestamp),
            _ => None,
        }
    }

    /// Returns the time when the transaction was locally marked as pending.
    /// If the transaction was not locally marked as pending, returns None.
    fn local_pending_timestamp(&self) -> Option<SystemTime> {
        let transaction_status_timeline = &self.authored_partial_trace.transaction_status_timeline;
        match transaction_status_timeline.get(&TransactionStatus::Pending) {
            Some(TransactionStatusMetadata::Timestamp(timestamp)) => Some(*timestamp),
            _ => {
                error!(
                    "Transaction was not marked as pending in the local partial trace: {:?}",
                    self.authored_partial_trace
                );
                None
            },
        }
    }

    /// Merges the transaction trace received from a peer into this trace
    pub fn merge_peer_trace(
        &mut self,
        peer_network_id: PeerNetworkId,
        peer_transaction_trace: TransactionTrace,
    ) {
        // Add the author's partial trace to our collection
        self.add_peer_partial_trace(
            peer_network_id,
            peer_transaction_trace.authored_partial_trace,
        );

        // Add the peer partial traces to our collection
        for (peer_network_id, peer_partial_trace) in peer_transaction_trace.peer_partial_traces {
            self.add_peer_partial_trace(peer_network_id, peer_partial_trace);
        }
    }
}

/// A partial trace of a transaction's activity in the system
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PartialTrace {
    // The peer that authored the transaction trace
    trace_author: PeerNetworkId,

    // Details about when and from where the transaction was received
    transaction_received_at: SystemTime,
    transaction_received_from: TransactionSource,

    // Details about when and to where the transaction was forwarded (if applicable)
    transaction_forwarded_at: Option<SystemTime>,
    transaction_forwarded_to: Option<TransactionDestination>,

    // The transaction status timeline (mapping from status to metadata)
    transaction_status_timeline: BTreeMap<TransactionStatus, TransactionStatusMetadata>,
}

impl PartialTrace {
    pub fn new(trace_author: PeerNetworkId, transaction_received_from: TransactionSource) -> Self {
        // Create the partial trace
        let mut partial_trace = Self {
            trace_author,
            transaction_status_timeline: BTreeMap::new(),
            transaction_received_at: SystemTime::now(),
            transaction_received_from,
            transaction_forwarded_at: None,
            transaction_forwarded_to: None,
        };

        // Update the timeline to include the pending state (all transactions start as pending)
        partial_trace.add_transaction_status_entry(
            TransactionStatus::Pending,
            TransactionStatusMetadata::Timestamp(SystemTime::now()),
        );

        partial_trace
    }

    /// Adds a new entry to the transaction status timeline
    fn add_transaction_status_entry(
        &mut self,
        transaction_status: TransactionStatus,
        transaction_status_metadata: TransactionStatusMetadata,
    ) {
        self.transaction_status_timeline
            .insert(transaction_status, transaction_status_metadata);
    }

    /// Marks the transaction as committed
    pub fn mark_transaction_committed(&mut self) {
        self.add_transaction_status_entry(
            TransactionStatus::Committed,
            TransactionStatusMetadata::Timestamp(SystemTime::now()),
        );
    }

    /// Marks the transaction as expired with the specified reason
    pub fn mark_transaction_expired(&mut self, expiration_reason: TransactionExpirationReason) {
        self.add_transaction_status_entry(
            TransactionStatus::Expired,
            TransactionStatusMetadata::TimestampAndExpiry(SystemTime::now(), expiration_reason),
        );
    }

    /// Marks the transaction as forwarded to the specified destination
    pub fn mark_transaction_forwarded(&mut self, destination: TransactionDestination) {
        self.transaction_forwarded_at = Some(SystemTime::now());
        self.transaction_forwarded_to = Some(destination);
    }

    /// Marks the transaction as pulled by quorum store
    pub fn mark_transaction_pulled_by_quorum_store(&mut self) {
        self.add_transaction_status_entry(
            TransactionStatus::PulledByQuorumStore,
            TransactionStatusMetadata::Timestamp(SystemTime::now()),
        );
    }

    /// Marks the transaction as rejected by quorum store
    pub fn mark_transaction_rejected_by_quorum_store(
        &mut self,
        rejection_reason: DiscardedVMStatus,
    ) {
        self.add_transaction_status_entry(
            TransactionStatus::RejectedByQuorumStore,
            TransactionStatusMetadata::TimestampAndRejection(SystemTime::now(), rejection_reason),
        );
    }

    /// Returns the author of the trace
    pub fn trace_author(&self) -> &PeerNetworkId {
        &self.trace_author
    }

    /// Returns the transaction destination (if it was forwarded)
    pub fn transaction_forwarded_to(&self) -> Option<TransactionDestination> {
        self.transaction_forwarded_to.clone()
    }

    /// Returns the source from which the transaction was received
    pub fn transaction_received_from(&self) -> &TransactionSource {
        &self.transaction_received_from
    }
}

/// The status of a transaction
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum TransactionStatus {
    Pending,               // Transactions is pending
    PulledByQuorumStore,   // Transaction was pulled by quorum store
    RejectedByQuorumStore, // Transaction was rejected by quorum store
    Committed,             // Transaction was successfully committed
    Expired,               // Transaction expired before confirmation
}

/// Any metadata associated with the transaction status
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum TransactionStatusMetadata {
    Timestamp(SystemTime), // Timestamp when the status was recorded
    TimestampAndExpiry(SystemTime, TransactionExpirationReason), // Timestamp and expiration reason
    TimestampAndRejection(SystemTime, DiscardedVMStatus), // Timestamp and rejection reason
}

/// The reason for a transaction expiration
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum TransactionExpirationReason {
    ClientExpiration,    // Expired due to client-specified expiration time
    SystemTTLExpiration, // Expired due to system time-to-live expiration
}

/// The source of a transaction (e.g., local REST API client or remote peer)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TransactionSource {
    RemotePeer(PeerNetworkId), // Transaction was received from a remote peer
    RestApi,                   // Transaction submitted by a client on the REST API
}

/// The destination of a transaction (e.g., a remote peer or quorum store)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TransactionDestination {
    RemotePeer(PeerNetworkId), // Transaction was forwarded to a remote peer
}

/// Returns the duration in seconds since the given timestamp.
/// If the timestamp is in the future, returns None.
fn duration_since_seconds(timestamp: SystemTime) -> Option<u64> {
    match SystemTime::now().duration_since(timestamp) {
        Ok(duration) => Some(duration.as_secs()),
        Err(error) => {
            error!(
                "Failed to compute duration since timestamp {:?}: {}",
                timestamp, error
            );
            None
        },
    }
}

/// Returns a string representation of the peer for use in transaction
/// traces. If the peer is known, returns its integer identifier as a
/// string, otherwise display the entire peer network ID.
fn get_peer_string_for_trace(
    peer_network_id: &PeerNetworkId,
    peer_network_id_to_int: &BTreeMap<PeerNetworkId, usize>,
) -> String {
    if let Some(authored_peer_int) = peer_network_id_to_int.get(peer_network_id) {
        format!("{}", authored_peer_int)
    } else {
        peer_network_id.to_string()
    }
}

/// Returns a compact string representation of the given transaction status timeline
pub fn get_timeline_string(
    transaction_status_timeline: &BTreeMap<TransactionStatus, TransactionStatusMetadata>,
) -> String {
    let mut transaction_timeline_strings = vec![];
    for (transaction_status, status_metadata) in transaction_status_timeline {
        // Get the string representation of the transaction status
        let status_string = match transaction_status {
            TransactionStatus::Pending => TRANSACTION_PENDING_STRING,
            TransactionStatus::PulledByQuorumStore => TRANSACTION_PULLED_BY_QUORUM_STORE_STRING,
            TransactionStatus::RejectedByQuorumStore => TRANSACTION_REJECTED_BY_QUORUM_STORE_STRING,
            TransactionStatus::Committed => TRANSACTION_COMMITTED_STRING,
            TransactionStatus::Expired => TRANSACTION_EXPIRED_STRING,
        };

        // Get the string representation of the transaction metadata (if any)
        let metadata_string = match status_metadata {
            TransactionStatusMetadata::Timestamp(timestamp) => Some(to_utc_time(timestamp)),
            TransactionStatusMetadata::TimestampAndExpiry(timestamp, expiration_reason) => Some(
                format!("{} ({:?})", to_utc_time(timestamp), expiration_reason),
            ),
            TransactionStatusMetadata::TimestampAndRejection(timestamp, rejection_reason) => Some(
                format!("{} ({:?})", to_utc_time(timestamp), rejection_reason),
            ),
        };

        // Combine the status string and metadata string
        let transaction_timeline_string = if let Some(metadata) = metadata_string {
            format!("{} ({})", status_string, metadata)
        } else {
            status_string.to_string()
        };
        transaction_timeline_strings.push(transaction_timeline_string);
    }

    // Return the final string
    transaction_timeline_strings.join(COMMA_DELIMITER_STRING)
}

/// Converts a system time to a human-readable UTC time string.
/// Note: we don't need to include the date in the output string,
/// as dates automatically included by the logging system.
fn to_utc_time(timestamp: &SystemTime) -> String {
    let datetime: DateTime<Utc> = (*timestamp).into();
    datetime.format("%H:%M:%S%.3f").to_string()
}
