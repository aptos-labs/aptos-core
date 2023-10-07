// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    global_summary::{AdvertisedData, GlobalDataSummary, OptimalChunkSizes},
    interface::ResponseError,
    logging::{LogEntry, LogEvent, LogSchema},
};
use aptos_config::{config::AptosDataClientConfig, network_id::PeerNetworkId};
use aptos_logger::prelude::*;
use aptos_storage_service_types::{
    requests::StorageServiceRequest, responses::StorageServerSummary,
};
use aptos_time_service::TimeService;
use dashmap::DashMap;
use std::{cmp::min, collections::HashSet, sync::Arc};

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
            },
            ResponseError::ProofVerificationError => ErrorType::Malicious,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PeerState {
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
    pub(crate) fn get_storage_summary_if_not_ignored(&self) -> Option<&StorageServerSummary> {
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
#[derive(Clone, Debug)]
pub(crate) struct PeerStates {
    data_client_config: Arc<AptosDataClientConfig>,
    peer_to_state: Arc<DashMap<PeerNetworkId, PeerState>>,
}

impl PeerStates {
    pub fn new(data_client_config: Arc<AptosDataClientConfig>) -> Self {
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
            .or_default()
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

    #[cfg(test)]
    /// Returns a copy of the peer to states map for test purposes
    pub fn get_peer_to_states(&self) -> Arc<DashMap<PeerNetworkId, PeerState>> {
        self.peer_to_state.clone()
    }
}

/// To calculate the optimal chunk size, we take the median for each
/// chunk size parameter. This works well when we have an honest
/// majority that mostly agrees on the same chunk sizes.
pub(crate) fn calculate_optimal_chunk_sizes(
    config: &AptosDataClientConfig,
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
