// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{trace::TransactionTrace, trace_collector::TransactionTraceCollector};
use aptos_config::{config::TransactionTracingConfig, network_id::PeerNetworkId};
use aptos_crypto::HashValue;
use aptos_logger::info;
use aptos_time_service::{TimeService, TimeServiceTrait};
use futures::StreamExt;
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
    time::Duration,
};

// Useful delimiter string constants
const COMMA_DELIMITER_STRING: &str = ", ";
const NEWLINE_DELIMITER_STRING: &str = "\n";

// The length to which transaction hashes are truncated for logging
const TRANSACTION_HASH_TRUNCATION_LENGTH: usize = 8;

/// A component for logging transaction traces in human-readable format
pub struct TransactionTraceLogger {
    // The time service
    time_service: TimeService,

    // The collector for transaction traces
    transaction_trace_collector: Arc<TransactionTraceCollector>,

    // The configuration for transaction tracing
    transaction_tracing_config: TransactionTracingConfig,
}

impl TransactionTraceLogger {
    pub fn new(
        time_service: TimeService,
        transaction_trace_collector: Arc<TransactionTraceCollector>,
        transaction_tracing_config: TransactionTracingConfig,
    ) -> Self {
        Self {
            time_service,
            transaction_trace_collector,
            transaction_tracing_config,
        }
    }

    /// Logs the completed transaction traces in human-readable format
    pub fn log_completed_traces(&self) {
        //info!("Logging completed transaction traces...");

        // Get all complete or expired transaction traces
        let (
            complete_transaction_traces,
            expired_transaction_traces,
            garbage_collected_transaction_traces,
        ) = self
            .transaction_trace_collector
            .get_complete_expired_and_garbage_collected_traces();

        // Verify we have transaction traces to log
        if complete_transaction_traces.is_empty()
            && expired_transaction_traces.is_empty()
            && garbage_collected_transaction_traces.is_empty()
        {
            //info!("No completed or expired transaction traces to log!");
            return; // Nothing to log
        } else {
            info!(
                "Found {} completed, {} expired, and {} garbage collected transaction traces to log.",
                complete_transaction_traces.len(),
                expired_transaction_traces.len(),
                garbage_collected_transaction_traces.len(),
            );
        }

        // Go through all traces and gather the peer IDs
        let all_transaction_traces = complete_transaction_traces
            .iter()
            .chain(expired_transaction_traces.iter())
            .chain(garbage_collected_transaction_traces.iter());
        let mut peer_network_ids = HashSet::new();
        for transaction_trace_entry in all_transaction_traces {
            let transaction_trace = transaction_trace_entry.value();

            // Insert the author peer network ID
            let author_peer_network_id = transaction_trace
                .get_authored_partial_trace()
                .trace_author();
            peer_network_ids.insert(*author_peer_network_id);

            // Insert all network IDs from peer partial traces
            for peer_network_id in transaction_trace.get_peer_partial_traces().keys() {
                peer_network_ids.insert(*peer_network_id);
            }
        }

        // Build a map from peer network IDs to unique integers
        let mut peer_network_id_to_int = BTreeMap::new();
        for (index, peer_network_id) in peer_network_ids.iter().enumerate() {
            peer_network_id_to_int.insert(*peer_network_id, index);
        }

        // Build a peer ID string map (for compact logging)
        let mut peer_network_id_string_map = vec![];
        for (peer_network_id, index) in peer_network_id_to_int.iter() {
            let string = format!("{} ({})", index, peer_network_id);
            peer_network_id_string_map.push(string);
        }

        // Go through all complete traces and generate a compact string
        let mut complete_trace_strings = vec![];
        for (transaction_hash, transaction_trace) in complete_transaction_traces {
            let trace_string = generate_compact_trace_string(
                &transaction_hash,
                transaction_trace,
                &peer_network_id_to_int,
            );
            complete_trace_strings.push(trace_string);
        }

        // Go through all expired traces and generate a compact string
        let mut expired_trace_strings = vec![];
        for (transaction_hash, transaction_trace) in expired_transaction_traces {
            let trace_string = generate_compact_trace_string(
                &transaction_hash,
                transaction_trace,
                &peer_network_id_to_int,
            );
            expired_trace_strings.push(trace_string);
        }

        // Go through all garbage collected traces and generate a compact string
        let mut garbage_collected_trace_strings = vec![];
        for (transaction_hash, transaction_trace) in garbage_collected_transaction_traces {
            let trace_string = generate_compact_trace_string(
                &transaction_hash,
                transaction_trace,
                &peer_network_id_to_int,
            );
            garbage_collected_trace_strings.push(trace_string);
        }

        // Collect the final log strings
        let mut final_trace_log_strings = vec![];
        final_trace_log_strings.push(format!(
            "Peers:\n {}",
            peer_network_id_string_map.join(COMMA_DELIMITER_STRING)
        ));
        if !complete_trace_strings.is_empty() {
            final_trace_log_strings.push(format!(
                "Committed:\n {}",
                complete_trace_strings.join(NEWLINE_DELIMITER_STRING)
            ));
        }
        if !expired_trace_strings.is_empty() {
            final_trace_log_strings.push(format!(
                "Expired:\n {}",
                expired_trace_strings.join(NEWLINE_DELIMITER_STRING)
            ));
        }
        if !garbage_collected_trace_strings.is_empty() {
            final_trace_log_strings.push(format!(
                "Garbage Collected:\n {}",
                garbage_collected_trace_strings.join(NEWLINE_DELIMITER_STRING)
            ));
        }

        // Log the transaction traces
        info!("{}", final_trace_log_strings.join(NEWLINE_DELIMITER_STRING));
    }

    /// Starts the transaction trace logger thread
    pub async fn start(self) {
        info!("Starting the transaction trace logger thread...");

        // Create a ticker for periodic logging
        let logging_duration = Duration::from_millis(
            self.transaction_tracing_config
                .trace_logging_loop_interval_secs,
        );
        let logging_ticker = self.time_service.interval(logging_duration);
        futures::pin_mut!(logging_ticker);

        // Continuously run the trace logger
        loop {
            futures::select! {
                _ = logging_ticker.select_next_some() => {
                    self.log_completed_traces();
                },
            }
        }
    }
}

/// Generates a compact string representation of a transaction trace
fn generate_compact_trace_string(
    transaction_hash: &HashValue,
    transaction_trace: TransactionTrace,
    peer_network_id_to_int: &BTreeMap<PeerNetworkId, usize>,
) -> String {
    // Gather all necessary components for the trace string
    let truncated_hash = &transaction_hash.to_hex()[..TRANSACTION_HASH_TRUNCATION_LENGTH];
    let transaction_route_string =
        transaction_trace.get_transaction_route_string(peer_network_id_to_int);
    let local_timeline_string = transaction_trace.get_local_timeline_string();
    let peer_timeline_string = transaction_trace.get_peer_timeline_string(peer_network_id_to_int);

    // Generate the final trace string
    format!(
        "Txn: {}, Route: {}, Local Timeline (UTC): {}, Peer Timelines (UTC): {}",
        truncated_hash, transaction_route_string, local_timeline_string, peer_timeline_string
    )
}
