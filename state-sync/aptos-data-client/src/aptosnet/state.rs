// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptosnet::logging::{LogEntry, LogEvent, LogSchema},
    AdvertisedData, GlobalDataSummary, OptimalChunkSizes, ResponseError,
};
use aptos_config::{
    config::{BaseConfig, StorageServiceConfig},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_logger::prelude::*;
use itertools::Itertools;
use netcore::transport::ConnectionOrigin;
use network::application::storage::PeerMetadataStorage;
use std::{
    cmp::min,
    collections::{HashMap, HashSet},
    sync::Arc,
};
use storage_service_types::requests::StorageServiceRequest;
use storage_service_types::responses::StorageServerSummary;

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

pub(crate) enum ErrorType {
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
            }
            ResponseError::ProofVerificationError => ErrorType::Malicious,
        }
    }
}

#[derive(Debug)]
struct PeerState {
    /// The latest observed advertised data for this peer, or `None` if we
    /// haven't polled them yet.
    storage_summary: Option<StorageServerSummary>,
    /// For now, a simplified port of the original state-sync v1 scoring system.
    score: f64,
}

impl Default for PeerState {
    fn default() -> Self {
        Self {
            storage_summary: None,
            score: STARTING_SCORE,
        }
    }
}

impl PeerState {
    /// Updates the storage summary for the peer
    fn update_storage_summary(&mut self, storage_summary: StorageServerSummary) {
        self.storage_summary = Some(storage_summary);
    }

    /// Returns the storage summary iff the peer is not below the ignore threshold
    fn storage_summary_if_not_ignored(&self) -> Option<&StorageServerSummary> {
        if self.score <= IGNORE_PEER_THRESHOLD {
            None
        } else {
            self.storage_summary.as_ref()
        }
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
}

/// Contains all of the unbanned peers' most recent [`StorageServerSummary`] data
/// advertisements and data-client internal metadata for scoring.
// TODO(philiphayes): this map needs to be garbage collected
#[derive(Debug)]
pub(crate) struct PeerStates {
    base_config: BaseConfig,
    storage_service_config: StorageServiceConfig,
    peer_to_state: HashMap<PeerNetworkId, PeerState>,
    in_flight_priority_polls: HashSet<PeerNetworkId>, // The priority peers with in-flight polls
    in_flight_regular_polls: HashSet<PeerNetworkId>,  // The regular peers with in-flight polls
    peer_metadata_storage: Arc<PeerMetadataStorage>,
}

impl PeerStates {
    pub fn new(
        base_config: BaseConfig,
        storage_service_config: StorageServiceConfig,
        peer_metadata_storage: Arc<PeerMetadataStorage>,
    ) -> Self {
        Self {
            base_config,
            storage_service_config,
            peer_to_state: HashMap::new(),
            in_flight_priority_polls: HashSet::new(),
            in_flight_regular_polls: HashSet::new(),
            peer_metadata_storage,
        }
    }

    /// Returns true if a connected storage service peer can actually fulfill a
    /// request, given our current view of their advertised data summary.
    pub fn can_service_request(
        &self,
        peer: &PeerNetworkId,
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

        self.peer_to_state
            .get(peer)
            .and_then(PeerState::storage_summary_if_not_ignored)
            .map(|summary| summary.can_service(request))
            .unwrap_or(false)
    }

    /// Updates the score of the peer according to a successful operation
    pub fn update_score_success(&mut self, peer: PeerNetworkId) {
        let old_score = self.peer_to_state.entry(peer).or_default().score;
        self.peer_to_state
            .entry(peer)
            .or_default()
            .update_score_success();
        let new_score = self.peer_to_state.entry(peer).or_default().score;
        if old_score <= IGNORE_PEER_THRESHOLD && new_score > IGNORE_PEER_THRESHOLD {
            info!(
                (LogSchema::new(LogEntry::PeerStates)
                    .event(LogEvent::PeerNoLongerIgnored)
                    .message("Peer will no longer be ignored")
                    .peer(&peer))
            );
        }
    }

    /// Updates the score of the peer according to an error
    pub fn update_score_error(&mut self, peer: PeerNetworkId, error: ErrorType) {
        let old_score = self.peer_to_state.entry(peer).or_default().score;
        self.peer_to_state
            .entry(peer)
            .or_default()
            .update_score_error(error);
        let new_score = self.peer_to_state.entry(peer).or_default().score;
        if old_score > IGNORE_PEER_THRESHOLD && new_score <= IGNORE_PEER_THRESHOLD {
            info!(
                (LogSchema::new(LogEntry::PeerStates)
                    .event(LogEvent::PeerIgnored)
                    .message("Peer will be ignored")
                    .peer(&peer))
            );
        }
    }

    /// Returns the number of in-flight priority polls
    pub fn num_in_flight_priority_polls(&self) -> u64 {
        self.in_flight_priority_polls.len() as u64
    }

    /// Returns the number of in-flight regular polls
    pub fn num_in_flight_regular_polls(&self) -> u64 {
        self.in_flight_regular_polls.len() as u64
    }

    /// Returns true iff there is an existing in-flight request
    pub fn existing_in_flight_request(&self, peer: &PeerNetworkId) -> bool {
        self.in_flight_priority_polls.contains(peer) || self.in_flight_regular_polls.contains(peer)
    }

    /// Marks an in-flight request as started for the specified peer
    pub fn new_in_flight_request(&mut self, peer: &PeerNetworkId) {
        // Get the current in-flight polls
        let is_priority_peer = self.is_priority_peer(peer);
        let in_flight_polls = if is_priority_peer {
            &mut self.in_flight_priority_polls
        } else {
            &mut self.in_flight_regular_polls
        };

        // Insert the new peer
        if !in_flight_polls.insert(*peer) {
            error!(
                (LogSchema::new(LogEntry::PeerStates)
                    .event(LogEvent::PriorityAndRegularPeers)
                    .message(&format!(
                        "Peer already found with an in-flight poll! Priority: {:?}",
                        is_priority_peer
                    ))
                    .peer(peer))
            );
        }
    }

    /// Marks the pending in-flight request as complete for the specified peer
    pub fn mark_in_flight_request_complete(&mut self, peer: &PeerNetworkId) {
        // The priority of the peer might have changed since we
        // last polled it, so we attempt to remove it from both
        // the regular and priority in-flight requests.
        if !self.in_flight_priority_polls.remove(peer) && !self.in_flight_regular_polls.remove(peer)
        {
            error!(
                (LogSchema::new(LogEntry::PeerStates)
                    .event(LogEvent::PriorityAndRegularPeers)
                    .message("Peer not found with an in-flight poll!")
                    .peer(peer))
            );
        }
    }

    /// Returns true iff the given peer is high-priority.
    ///
    /// TODO(joshlind): make this less hacky using network topological awareness.
    pub fn is_priority_peer(&self, peer: &PeerNetworkId) -> bool {
        // Validators should only prioritize other validators
        let peer_network_id = peer.network_id();
        if self.base_config.role.is_validator() {
            return peer_network_id.is_validator_network();
        }

        // VFNs should only prioritize validators
        if self
            .peer_metadata_storage
            .networks()
            .contains(&NetworkId::Vfn)
        {
            return peer_network_id.is_vfn_network();
        }

        // PFNs should only prioritize outbound connections (this targets seed peers and VFNs)
        if let Some(peer_info) = self.peer_metadata_storage.read(*peer) {
            if peer_info.active_connection.origin == ConnectionOrigin::Outbound {
                return true;
            }
        }

        false
    }

    /// Updates the storage summary for the given peer
    pub fn update_summary(&mut self, peer: PeerNetworkId, summary: StorageServerSummary) {
        self.peer_to_state
            .entry(peer)
            .or_default()
            .update_storage_summary(summary);
    }

    /// Calculates a global data summary using all known storage summaries
    pub fn calculate_aggregate_summary(&self) -> GlobalDataSummary {
        // Only include likely-not-malicious peers in the data summary aggregation
        let summaries: Vec<StorageServerSummary> = self
            .peer_to_state
            .values()
            .filter_map(PeerState::storage_summary_if_not_ignored)
            .cloned()
            .collect();

        // If we have no peers, return an empty global summary
        if summaries.is_empty() {
            return GlobalDataSummary::empty();
        }

        // Calculate the global data summary using the advertised peer data
        let mut advertised_data = AdvertisedData::empty();
        let mut max_epoch_chunk_sizes = vec![];
        let mut max_state_chunk_sizes = vec![];
        let mut max_transaction_chunk_sizes = vec![];
        let mut max_transaction_output_chunk_sizes = vec![];
        for summary in summaries {
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
            &self.storage_service_config,
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
}

/// To calculate the optimal chunk size, we take the median for each
/// chunk size parameter. This works well when we have an honest
/// majority that mostly agrees on the same chunk sizes.
pub(crate) fn calculate_optimal_chunk_sizes(
    config: &StorageServiceConfig,
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
