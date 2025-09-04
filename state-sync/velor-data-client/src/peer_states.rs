// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    global_summary::{AdvertisedData, GlobalDataSummary, OptimalChunkSizes},
    interface::ResponseError,
    logging::{LogEntry, LogEvent, LogSchema},
    metrics,
};
use velor_config::{
    config::VelorDataClientConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use velor_logger::prelude::*;
use velor_storage_service_types::{
    requests::StorageServiceRequest, responses::StorageServerSummary,
};
use velor_time_service::TimeService;
use dashmap::DashMap;
use std::{
    cmp::min,
    collections::{BTreeMap, HashSet},
    sync::Arc,
    time::Duration,
};

// Useful constants
const LOGS_FREQUENCY_SECS: u64 = 120; // 2 minutes
const METRICS_FREQUENCY_SECS: u64 = 15; // 15 seconds
const NUM_PEER_BUCKETS_FOR_METRICS: u8 = 4; // To avoid metric explosion, we bucket peers into groups

/// Scores for peer rankings based on preferences and behavior.
const MAX_SCORE: f64 = 100.0;
const MIN_SCORE: f64 = 0.0;
const STARTING_SCORE: f64 = 50.0;
/// Add this score on a successful response.
const SUCCESSFUL_RESPONSE_DELTA: f64 = 1.0;
/// Not necessarily a malicious response, but not super useful.
const NOT_USEFUL_MULTIPLIER: f64 = 0.95;
/// Likely to be a malicious response.
const MALICIOUS_MULTIPLIER: f64 = 0.8;
/// Ignore a peer when their score dips below this threshold.
const IGNORE_PEER_THRESHOLD: f64 = 25.0;

pub enum ErrorType {
    /// A response or error that's not actively malicious but also doesn't help
    /// us make progress, e.g., timeouts, remote errors, invalid data, etc...
    NotUseful,
    /// A response or error that appears to be actively hindering progress or
    /// attempting to deceive us, e.g., invalid proof.
    Malicious,
}

impl From<ResponseError> for ErrorType {
    fn from(error: ResponseError) -> Self {
        match error {
            ResponseError::InvalidData | ResponseError::InvalidPayloadDataType => {
                ErrorType::NotUseful
            },
            ResponseError::ProofVerificationError => ErrorType::Malicious,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PeerState {
    /// The data client configuration
    data_client_config: Arc<VelorDataClientConfig>,

    /// The number of responses received from this peer (by data request label)
    received_responses_by_type: Arc<DashMap<String, u64>>,

    /// The number of requests sent to this peer (by data request label)
    sent_requests_by_type: Arc<DashMap<String, u64>>,

    /// The latest observed advertised data for this peer, or `None` if we
    /// haven't polled them yet.
    storage_summary: Option<StorageServerSummary>,

    /// For now, a simplified port of the original state-sync v1 scoring system.
    score: f64,
}

impl PeerState {
    pub fn new(data_client_config: Arc<VelorDataClientConfig>) -> Self {
        Self {
            data_client_config,
            received_responses_by_type: Arc::new(DashMap::new()),
            sent_requests_by_type: Arc::new(DashMap::new()),
            storage_summary: None,
            score: STARTING_SCORE,
        }
    }
}

impl PeerState {
    /// Increments the received response counter for the given label
    fn increment_received_response_counter(&mut self, response_label: String) {
        self.received_responses_by_type
            .entry(response_label)
            .and_modify(|counter| *counter += 1)
            .or_insert(1);
    }

    /// Increments the sent request counter for the given label
    fn increment_sent_request_counter(&mut self, request_label: String) {
        self.sent_requests_by_type
            .entry(request_label)
            .and_modify(|counter| *counter += 1)
            .or_insert(1);
    }

    /// Returns the peer's score
    pub fn get_score(&self) -> f64 {
        self.score
    }

    /// Returns the storage summary for the peer
    pub fn get_storage_summary(&self) -> Option<StorageServerSummary> {
        self.storage_summary.clone()
    }

    /// Returns a sorted copy of the sent requests by type map
    pub fn get_sent_requests_by_type(&self) -> BTreeMap<String, u64> {
        let mut sorted_requests_by_type = BTreeMap::new();
        for sent_request in self.sent_requests_by_type.iter() {
            sorted_requests_by_type.insert(sent_request.key().clone(), *sent_request.value());
        }
        sorted_requests_by_type
    }

    /// Returns a sorted copy of the received responses by type map
    pub fn get_received_responses_by_type(&self) -> BTreeMap<String, u64> {
        let mut sorted_responses_by_type = BTreeMap::new();
        for received_response in self.received_responses_by_type.iter() {
            sorted_responses_by_type
                .insert(received_response.key().clone(), *received_response.value());
        }
        sorted_responses_by_type
    }

    /// Returns the storage summary iff the peer is not below the ignore threshold
    pub fn get_storage_summary_if_not_ignored(&self) -> Option<&StorageServerSummary> {
        if self.is_ignored() {
            None
        } else {
            self.storage_summary.as_ref()
        }
    }

    /// Returns true iff the peer is currently ignored
    fn is_ignored(&self) -> bool {
        // Only ignore peers if the config allows it
        if !self.data_client_config.ignore_low_score_peers {
            return false;
        }

        // Otherwise, ignore peers with a low score
        self.score <= IGNORE_PEER_THRESHOLD
    }

    /// Updates the score of the peer according to a successful operation
    fn update_score_success(&mut self) {
        self.score = f64::min(self.score + SUCCESSFUL_RESPONSE_DELTA, MAX_SCORE);
    }

    /// Updates the score of the peer according to an error
    fn update_score_error(&mut self, error: ErrorType) {
        let multiplier = match error {
            ErrorType::NotUseful => NOT_USEFUL_MULTIPLIER,
            ErrorType::Malicious => MALICIOUS_MULTIPLIER,
        };
        self.score = f64::max(self.score * multiplier, MIN_SCORE);
    }

    /// Updates the storage summary for the peer
    fn update_storage_summary(&mut self, storage_summary: StorageServerSummary) {
        self.storage_summary = Some(storage_summary);
    }
}

/// Contains all of the unbanned peers' most recent [`StorageServerSummary`] data
/// advertisements and data-client internal metadata for scoring.
#[derive(Clone, Debug)]
pub struct PeerStates {
    data_client_config: Arc<VelorDataClientConfig>,
    peer_to_state: Arc<DashMap<PeerNetworkId, PeerState>>,
}

impl PeerStates {
    pub fn new(data_client_config: Arc<VelorDataClientConfig>) -> Self {
        Self {
            data_client_config,
            peer_to_state: Arc::new(DashMap::new()),
        }
    }

    /// Returns true if a connected storage service peer can actually fulfill a
    /// request, given our current view of their advertised data summary.
    pub fn can_service_request(
        &self,
        peer: &PeerNetworkId,
        time_service: TimeService,
        request: &StorageServiceRequest,
    ) -> bool {
        // Storage services can always respond to data advertisement requests.
        // We need this outer check, since we need to be able to send data summary
        // requests to new peers (who don't have a peer state yet).
        if request.data_request.is_storage_summary_request()
            || request.data_request.is_protocol_version_request()
        {
            return true;
        }

        // Check if the peer can service the request
        if let Some(peer_state) = self.peer_to_state.get(peer) {
            return match peer_state.get_storage_summary_if_not_ignored() {
                Some(storage_summary) => {
                    storage_summary.can_service(&self.data_client_config, time_service, request)
                },
                None => false, // The peer is temporarily ignored
            };
        }

        // Otherwise, the request cannot be serviced
        false
    }

    /// Increments the received response counter for the given peer
    pub fn increment_received_response_counter(
        &self,
        peer: PeerNetworkId,
        request: &StorageServiceRequest,
    ) {
        // Get the data request label
        let response_label = request.data_request.get_label().into();

        // Update the peer's counter
        if let Some(mut entry) = self.peer_to_state.get_mut(&peer) {
            entry.increment_received_response_counter(response_label);
        }
    }

    /// Increments the sent request counter for the given peer
    pub fn increment_sent_request_counter(
        &self,
        peer: PeerNetworkId,
        request: &StorageServiceRequest,
    ) {
        // Get the data request label
        let request_label = request.data_request.get_label().into();

        // Update the peer's counter
        if let Some(mut entry) = self.peer_to_state.get_mut(&peer) {
            entry.increment_sent_request_counter(request_label);
        }
    }

    /// Updates the logs and metrics for the peer request distributions
    pub fn update_peer_request_logs_and_metrics(&self) {
        // Periodically update the metrics
        sample!(
            SampleRate::Duration(Duration::from_secs(METRICS_FREQUENCY_SECS)),
            update_peer_request_metrics(self.peer_to_state.clone());
        );

        // Periodically update the logs
        sample!(
            SampleRate::Duration(Duration::from_secs(LOGS_FREQUENCY_SECS)),
            update_peer_request_logs(self.peer_to_state.clone());
        );

        // Periodically update the metrics for ignored peers
        sample!(
            SampleRate::Duration(Duration::from_secs(METRICS_FREQUENCY_SECS)),
            update_peer_ignored_metrics(self.peer_to_state.clone());
        );
    }

    /// Updates the score of the peer according to a successful operation
    pub fn update_score_success(&self, peer: PeerNetworkId) {
        if let Some(mut entry) = self.peer_to_state.get_mut(&peer) {
            // Get the peer's old score
            let old_score = entry.score;

            // Update the peer's score with a successful operation
            entry.update_score_success();

            // Log if the peer is no longer ignored
            let new_score = entry.score;
            if old_score <= IGNORE_PEER_THRESHOLD && new_score > IGNORE_PEER_THRESHOLD {
                info!(
                    (LogSchema::new(LogEntry::PeerStates)
                        .event(LogEvent::PeerNoLongerIgnored)
                        .message("Peer will no longer be ignored")
                        .peer(&peer))
                );
            }
        }
    }

    /// Updates the score of the peer according to an error
    pub fn update_score_error(&self, peer: PeerNetworkId, error: ErrorType) {
        if let Some(mut entry) = self.peer_to_state.get_mut(&peer) {
            // Get the peer's old score
            let old_score = entry.score;

            // Update the peer's score with an error
            entry.update_score_error(error);

            // Log if the peer is now ignored
            let new_score = entry.score;
            if old_score > IGNORE_PEER_THRESHOLD && new_score <= IGNORE_PEER_THRESHOLD {
                info!(
                    (LogSchema::new(LogEntry::PeerStates)
                        .event(LogEvent::PeerIgnored)
                        .message("Peer will be ignored")
                        .peer(&peer))
                );
            }
        }
    }

    /// Updates the storage summary for the given peer
    pub fn update_summary(&self, peer: PeerNetworkId, storage_summary: StorageServerSummary) {
        self.peer_to_state
            .entry(peer)
            .or_insert(PeerState::new(self.data_client_config.clone()))
            .update_storage_summary(storage_summary);
    }

    /// Garbage collects the peer states to remove data for disconnected peers
    pub fn garbage_collect_peer_states(&self, connected_peers: HashSet<PeerNetworkId>) {
        self.peer_to_state
            .retain(|peer_network_id, _| connected_peers.contains(peer_network_id));
    }

    /// Calculates a global data summary using all known storage summaries
    pub fn calculate_global_data_summary(&self) -> GlobalDataSummary {
        // Gather all storage summaries, but exclude peers that are ignored
        let storage_summaries: Vec<StorageServerSummary> = self
            .peer_to_state
            .iter()
            .filter_map(|peer_state| {
                peer_state
                    .value()
                    .get_storage_summary_if_not_ignored()
                    .cloned()
            })
            .collect();

        // If we have no peers, return an empty global summary
        if storage_summaries.is_empty() {
            return GlobalDataSummary::empty();
        }

        // Calculate the global data summary using the advertised peer data
        let mut advertised_data = AdvertisedData::empty();
        let mut max_epoch_chunk_sizes = vec![];
        let mut max_state_chunk_sizes = vec![];
        let mut max_transaction_chunk_sizes = vec![];
        let mut max_transaction_output_chunk_sizes = vec![];
        for summary in storage_summaries {
            // Collect aggregate data advertisements
            if let Some(epoch_ending_ledger_infos) = summary.data_summary.epoch_ending_ledger_infos
            {
                advertised_data
                    .epoch_ending_ledger_infos
                    .push(epoch_ending_ledger_infos);
            }
            if let Some(states) = summary.data_summary.states {
                advertised_data.states.push(states);
            }
            if let Some(synced_ledger_info) = summary.data_summary.synced_ledger_info.as_ref() {
                advertised_data
                    .synced_ledger_infos
                    .push(synced_ledger_info.clone());
            }
            if let Some(transactions) = summary.data_summary.transactions {
                advertised_data.transactions.push(transactions);
            }
            if let Some(transaction_outputs) = summary.data_summary.transaction_outputs {
                advertised_data
                    .transaction_outputs
                    .push(transaction_outputs);
            }

            // Collect preferred max chunk sizes
            max_epoch_chunk_sizes.push(summary.protocol_metadata.max_epoch_chunk_size);
            max_state_chunk_sizes.push(summary.protocol_metadata.max_state_chunk_size);
            max_transaction_chunk_sizes.push(summary.protocol_metadata.max_transaction_chunk_size);
            max_transaction_output_chunk_sizes
                .push(summary.protocol_metadata.max_transaction_output_chunk_size);
        }

        // Calculate optimal chunk sizes based on the advertised data
        let optimal_chunk_sizes = calculate_optimal_chunk_sizes(
            &self.data_client_config,
            max_epoch_chunk_sizes,
            max_state_chunk_sizes,
            max_transaction_chunk_sizes,
            max_transaction_output_chunk_sizes,
        );
        GlobalDataSummary {
            advertised_data,
            optimal_chunk_sizes,
        }
    }

    /// Returns the peer to states map
    pub fn get_peer_to_states(&self) -> Arc<DashMap<PeerNetworkId, PeerState>> {
        self.peer_to_state.clone()
    }
}

/// To calculate the optimal chunk size, we take the median for each
/// chunk size parameter. This works well when we have an honest
/// majority that mostly agrees on the same chunk sizes.
pub(crate) fn calculate_optimal_chunk_sizes(
    config: &VelorDataClientConfig,
    max_epoch_chunk_sizes: Vec<u64>,
    max_state_chunk_sizes: Vec<u64>,
    max_transaction_chunk_sizes: Vec<u64>,
    max_transaction_output_chunk_size: Vec<u64>,
) -> OptimalChunkSizes {
    let epoch_chunk_size = median_or_max(max_epoch_chunk_sizes, config.max_epoch_chunk_size);
    let state_chunk_size = median_or_max(max_state_chunk_sizes, config.max_state_chunk_size);
    let transaction_chunk_size = median_or_max(
        max_transaction_chunk_sizes,
        config.max_transaction_chunk_size,
    );
    let transaction_output_chunk_size = median_or_max(
        max_transaction_output_chunk_size,
        config.max_transaction_output_chunk_size,
    );

    OptimalChunkSizes {
        epoch_chunk_size,
        state_chunk_size,
        transaction_chunk_size,
        transaction_output_chunk_size,
    }
}

/// Calculates the median of the given set of values (if it exists)
/// and returns the median or the specified max value, whichever is
/// lower.
fn median_or_max<T: Ord + Copy>(mut values: Vec<T>, max_value: T) -> T {
    // Calculate median
    values.sort_unstable();
    let idx = values.len() / 2;
    let median = values.get(idx).copied();

    // Return median or max
    min(median.unwrap_or(max_value), max_value)
}

/// Returns the bucket ID for the given peer. This is useful
/// for grouping peers together to avoid metric explosion.
pub fn get_bucket_id_for_peer(peer: PeerNetworkId) -> u8 {
    let peer_id_bytes = peer.peer_id().into_bytes();
    peer_id_bytes[0] % NUM_PEER_BUCKETS_FOR_METRICS
}

/// Updates the metrics for the number of ignored peers
fn update_peer_ignored_metrics(peer_to_state: Arc<DashMap<PeerNetworkId, PeerState>>) {
    // Collect the ignored peer counts by network
    let mut ignored_peer_counts_by_network: BTreeMap<NetworkId, u64> = BTreeMap::new();
    for peer_state_entry in peer_to_state.iter() {
        // Get the peer and state
        let peer = *peer_state_entry.key();
        let network_id = peer.network_id();
        let peer_state = peer_state_entry.value();

        // Get (or initialize) the network count entry
        let network_count_entry = ignored_peer_counts_by_network
            .entry(network_id)
            .or_default();

        // If the peer is ignored, increment the count
        if peer_state.is_ignored() {
            *network_count_entry += 1;
        }
    }

    // Update the ignored peer metrics
    for (network_id, ignored_peer_count) in ignored_peer_counts_by_network.iter() {
        metrics::set_gauge(
            &metrics::IGNORED_PEERS,
            &network_id.to_string(),
            *ignored_peer_count,
        );
    }
}

/// Updates the logs for the peer requests and responses by bucket
fn update_peer_request_logs(peer_to_state: Arc<DashMap<PeerNetworkId, PeerState>>) {
    // Collect the peer request and response counts
    let mut request_and_response_counts = vec![];
    for peer_state_entry in peer_to_state.iter() {
        // Get the peer and request data
        let peer = *peer_state_entry.key();
        let peer_bucket_id = get_bucket_id_for_peer(peer);
        let sent_requests_by_type = peer_state_entry.get_sent_requests_by_type();
        let received_responses_by_type = peer_state_entry.get_received_responses_by_type();

        // Collect the request and response counts
        let peer_and_requests_string = format!(
            "Peer: {:?}, Bucket ID: {:?}, Sent request counts: {:?}, Received response counts: {:?}",
            peer, peer_bucket_id, sent_requests_by_type, received_responses_by_type
        );
        request_and_response_counts.push(peer_and_requests_string);
    }

    // Log the peer request and response counts
    info!(LogSchema::new(LogEntry::PeerStates)
        .event(LogEvent::PeerRequestResponseCounts)
        .message(&format!(
            "Peer request and response counts: {:?}",
            request_and_response_counts
        )));
}

/// Updates the metrics for the peer requests and responses by bucket
fn update_peer_request_metrics(peer_to_state: Arc<DashMap<PeerNetworkId, PeerState>>) {
    // Aggregate all request and response counts by peer bucket
    let mut sent_requests_by_peer_bucket: BTreeMap<u8, BTreeMap<String, u64>> = BTreeMap::new();
    let mut received_responses_by_peer_bucket: BTreeMap<u8, BTreeMap<String, u64>> =
        BTreeMap::new();
    for peer_state_entry in peer_to_state.iter() {
        // Get the peer and request data
        let peer = *peer_state_entry.key();
        let peer_bucket_id = get_bucket_id_for_peer(peer);
        let sent_requests_by_type = peer_state_entry.get_sent_requests_by_type();
        let received_responses_by_type = peer_state_entry.get_received_responses_by_type();

        // Aggregate the sent request counts by peer bucket
        let sent_requests_by_bucket = sent_requests_by_peer_bucket
            .entry(peer_bucket_id)
            .or_default();
        for (request_label, count) in sent_requests_by_type.iter() {
            *sent_requests_by_bucket
                .entry(request_label.clone())
                .or_default() += count;
        }

        // Aggregate the received response counts by peer bucket
        let received_responses_by_bucket = received_responses_by_peer_bucket
            .entry(peer_bucket_id)
            .or_default();
        for (response_label, count) in received_responses_by_type.iter() {
            *received_responses_by_bucket
                .entry(response_label.clone())
                .or_default() += count;
        }
    }

    // Update the sent request metrics
    for (peer_bucket_id, sent_requests_by_type) in sent_requests_by_peer_bucket.iter() {
        for (request_label, count) in sent_requests_by_type.iter() {
            metrics::set_gauge_for_bucket(
                &metrics::SENT_REQUESTS_BY_PEER_BUCKET,
                &peer_bucket_id.to_string(),
                request_label,
                *count,
            );
        }
    }

    // Update the received response metrics
    for (peer_bucket_id, received_responses_by_type) in received_responses_by_peer_bucket.iter() {
        for (response_label, count) in received_responses_by_type.iter() {
            metrics::set_gauge_for_bucket(
                &metrics::RECEIVED_RESPONSES_BY_PEER_BUCKET,
                &peer_bucket_id.to_string(),
                response_label,
                *count,
            );
        }
    }
}
