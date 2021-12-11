// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    diemnet::logging::{LogEntry, LogEvent, LogSchema},
    AdvertisedData, GlobalDataSummary, OptimalChunkSizes, ResponseError,
};
use diem_config::{config::StorageServiceConfig, network_id::PeerNetworkId};
use diem_logger::debug;
use std::collections::HashMap;
use storage_service_types::{StorageServerSummary, StorageServiceRequest};

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
    fn storage_summary_if_not_ignored(&self) -> Option<&StorageServerSummary> {
        if self.score <= IGNORE_PEER_THRESHOLD {
            None
        } else {
            self.storage_summary.as_ref()
        }
    }

    fn update_score_success(&mut self) {
        self.score = f64::min(self.score + SUCCESSFUL_RESPONSE_DELTA, MAX_SCORE);
    }

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
    config: StorageServiceConfig,
    inner: HashMap<PeerNetworkId, PeerState>,
}

impl PeerStates {
    pub fn new(config: StorageServiceConfig) -> Self {
        Self {
            config,
            inner: HashMap::new(),
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
        if request.is_get_storage_server_summary() {
            return true;
        }

        self.inner
            .get(peer)
            .and_then(PeerState::storage_summary_if_not_ignored)
            .map(|summary| summary.can_service(request))
            .unwrap_or(false)
    }

    pub fn update_score_success(&mut self, peer: PeerNetworkId) {
        let old_score = self.inner.entry(peer).or_default().score;
        self.inner.entry(peer).or_default().update_score_success();
        let new_score = self.inner.entry(peer).or_default().score;
        if old_score <= IGNORE_PEER_THRESHOLD && new_score > IGNORE_PEER_THRESHOLD {
            debug!(
                (LogSchema::new(LogEntry::PeerStates)
                    .event(LogEvent::PeerNoLongerIgnored)
                    .message("Peer will no longer be ignored")
                    .peer(&peer))
            );
        }
    }

    pub fn update_score_error(&mut self, peer: PeerNetworkId, error: ErrorType) {
        let old_score = self.inner.entry(peer).or_default().score;
        self.inner
            .entry(peer)
            .or_default()
            .update_score_error(error);
        let new_score = self.inner.entry(peer).or_default().score;
        if old_score > IGNORE_PEER_THRESHOLD && new_score <= IGNORE_PEER_THRESHOLD {
            debug!(
                (LogSchema::new(LogEntry::PeerStates)
                    .event(LogEvent::PeerIgnored)
                    .message("Peer will be ignored")
                    .peer(&peer))
            );
        }
    }

    pub fn update_summary(&mut self, peer: PeerNetworkId, summary: StorageServerSummary) {
        self.inner.entry(peer).or_default().storage_summary = Some(summary);
    }

    pub fn aggregate_summary(&self) -> GlobalDataSummary {
        let mut aggregate_data = AdvertisedData::empty();

        let mut max_epoch_chunk_sizes = vec![];
        let mut max_transaction_chunk_sizes = vec![];
        let mut max_transaction_output_chunk_sizes = vec![];
        let mut max_account_states_chunk_sizes = vec![];

        // only include likely-not-malicious peers in the data summary aggregation.
        let summaries = self
            .inner
            .values()
            .filter_map(PeerState::storage_summary_if_not_ignored);

        // collect each peer's protocol and data advertisements
        for summary in summaries {
            // collect aggregate data advertisements
            if let Some(account_states) = summary.data_summary.account_states {
                aggregate_data.account_states.push(account_states);
            }
            if let Some(epoch_ending_ledger_infos) = summary.data_summary.epoch_ending_ledger_infos
            {
                aggregate_data
                    .epoch_ending_ledger_infos
                    .push(epoch_ending_ledger_infos);
            }
            if let Some(synced_ledger_info) = summary.data_summary.synced_ledger_info.as_ref() {
                aggregate_data
                    .synced_ledger_infos
                    .push(synced_ledger_info.clone());
            }
            if let Some(transactions) = summary.data_summary.transactions {
                aggregate_data.transactions.push(transactions);
            }
            if let Some(transaction_outputs) = summary.data_summary.transaction_outputs {
                aggregate_data.transaction_outputs.push(transaction_outputs);
            }

            // collect preferred max chunk sizes
            max_epoch_chunk_sizes.push(summary.protocol_metadata.max_epoch_chunk_size);
            max_transaction_chunk_sizes.push(summary.protocol_metadata.max_transaction_chunk_size);
            max_transaction_output_chunk_sizes
                .push(summary.protocol_metadata.max_transaction_output_chunk_size);
            max_account_states_chunk_sizes
                .push(summary.protocol_metadata.max_account_states_chunk_size);
        }

        // take the median for each max chunk size parameter.
        // this works well when we have an honest majority that mostly agrees on
        // the same chunk sizes.
        let aggregate_chunk_sizes = OptimalChunkSizes {
            account_states_chunk_size: median(&mut max_account_states_chunk_sizes)
                .unwrap_or(self.config.max_account_states_chunk_sizes),
            epoch_chunk_size: median(&mut max_epoch_chunk_sizes)
                .unwrap_or(self.config.max_epoch_chunk_size),
            transaction_chunk_size: median(&mut max_transaction_chunk_sizes)
                .unwrap_or(self.config.max_transaction_chunk_size),
            transaction_output_chunk_size: median(&mut max_transaction_output_chunk_sizes)
                .unwrap_or(self.config.max_transaction_output_chunk_size),
        };

        GlobalDataSummary {
            advertised_data: aggregate_data,
            optimal_chunk_sizes: aggregate_chunk_sizes,
        }
    }
}

fn median<T: Ord + Copy>(values: &mut [T]) -> Option<T> {
    values.sort_unstable();
    let idx = values.len() / 2;
    values.get(idx).copied()
}
