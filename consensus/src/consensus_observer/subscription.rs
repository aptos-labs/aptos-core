// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    error::Error,
    logging::{LogEntry, LogSchema},
};
use aptos_config::{config::ConsensusObserverConfig, network_id::PeerNetworkId};
use aptos_logger::warn;
use aptos_network::application::metadata::PeerMetadata;
use aptos_storage_interface::DbReader;
use aptos_time_service::{TimeService, TimeServiceTrait};
use ordered_float::OrderedFloat;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::{Duration, Instant},
};

/// A single consensus observer subscription
pub struct ConsensusObserverSubscription {
    // The configuration of the consensus observer
    consensus_observer_config: ConsensusObserverConfig,

    // A handle to storage (used to read the latest state and check progress)
    db_reader: Arc<dyn DbReader>,

    // The peer network id of the active publisher
    peer_network_id: PeerNetworkId,

    // The timestamp of the last message received from the peer
    last_message_receive_time: Instant,

    // The highest synced version we've seen from storage (along with the time at which it was seen)
    highest_synced_version: (u64, Instant),

    // The time service (used to check the last message receive time)
    time_service: TimeService,
}

impl ConsensusObserverSubscription {
    pub fn new(
        consensus_observer_config: ConsensusObserverConfig,
        db_reader: Arc<dyn DbReader>,
        peer_network_id: PeerNetworkId,
        time_service: TimeService,
    ) -> Self {
        let time_now = time_service.now();

        Self {
            consensus_observer_config,
            db_reader,
            peer_network_id,
            last_message_receive_time: time_now,
            highest_synced_version: (0, time_now),
            time_service,
        }
    }

    /// Verifies that the subscription has not timed out based on the last
    /// received message time. Otherwise, an error is returned.
    pub fn check_subscription_timeout(&self) -> Result<(), Error> {
        // Calculate the duration since the last message
        let time_now = self.time_service.now();
        let duration_since_last_message = time_now.duration_since(self.last_message_receive_time);

        // Check if the subscription has timed out
        if duration_since_last_message
            > Duration::from_millis(self.consensus_observer_config.max_subscription_timeout_ms)
        {
            return Err(Error::SubscriptionTimeout(format!(
                "Subscription to peer: {} has timed out! No message received for: {:?}",
                self.peer_network_id, duration_since_last_message
            )));
        }

        Ok(())
    }

    /// Verifies that the DB is continuing to sync and commit new data
    pub fn check_syncing_progress(&mut self) -> Result<(), Error> {
        // Get the current synced version from storage
        let current_synced_version =
            self.db_reader
                .get_latest_ledger_info_version()
                .map_err(|error| {
                    Error::UnexpectedError(format!(
                        "Failed to read highest synced version: {:?}",
                        error
                    ))
                })?;

        // Verify that the synced version is increasing appropriately
        let (highest_synced_version, highest_version_timestamp) = self.highest_synced_version;
        if current_synced_version <= highest_synced_version {
            // The synced version hasn't increased. Check if we should terminate
            // the subscription based on the last time the highest synced version was seen.
            let duration_since_highest_seen = highest_version_timestamp.elapsed();
            if duration_since_highest_seen
                > Duration::from_millis(
                    self.consensus_observer_config.max_synced_version_timeout_ms,
                )
            {
                return Err(Error::SubscriptionTimeout(format!(
                    "The DB is not making sync progress! Highest synced version: {}, elapsed: {:?}",
                    highest_synced_version, duration_since_highest_seen
                )));
            }
        }

        // Update the highest synced version and time
        self.highest_synced_version = (current_synced_version, self.time_service.now());

        Ok(())
    }

    /// Returns the peer network id of the subscription
    pub fn get_peer_network_id(&self) -> PeerNetworkId {
        self.peer_network_id
    }

    /// Verifies the given message is from the expected peer
    pub fn verify_message_sender(&mut self, peer_network_id: &PeerNetworkId) -> Result<(), Error> {
        // Verify the message is from the expected peer
        if self.peer_network_id != *peer_network_id {
            return Err(Error::UnexpectedError(format!(
                "Received message from unexpected peer: {}! Subscribed to: {}",
                peer_network_id, self.peer_network_id
            )));
        }

        // Update the last message receive time
        self.last_message_receive_time = self.time_service.now();

        Ok(())
    }
}

/// Sorts the peers by distance from the validator set and latency.
/// We prioritize distance over latency as we want to avoid close
/// but not up-to-date peers. If peers don't have sufficient metadata
/// for sorting, they are given a lower priority.
pub fn sort_peers_by_distance_and_latency(
    peers_and_metadata: HashMap<PeerNetworkId, PeerMetadata>,
) -> Vec<PeerNetworkId> {
    // Group peers and latency weights by validator distance, i.e., distance -> [(peer, latency weight)]
    let mut peers_and_latencies_by_distance = BTreeMap::new();
    for (peer_network_id, peer_metadata) in peers_and_metadata {
        // Get the distance and latency for the peer
        let distance = get_distance_for_peer(&peer_network_id, &peer_metadata);
        let latency = get_latency_for_peer(&peer_network_id, &peer_metadata);

        // If the distance is not found, use the maximum distance
        let distance =
            distance.unwrap_or(aptos_peer_monitoring_service_types::MAX_DISTANCE_FROM_VALIDATORS);

        // Convert the latency to a weight
        let latency_weight = convert_latency_to_weight(latency);

        // Add the peer and latency weight to the distance group
        peers_and_latencies_by_distance
            .entry(distance)
            .or_insert_with(Vec::new)
            .push((peer_network_id, OrderedFloat(latency_weight)));
    }

    // Sort the peers by distance and latency weights. Note: BTreeMaps are
    // sorted by key, so the entries will be sorted by distance in ascending order.
    let mut sorted_peers = Vec::new();
    for (_, mut peers_and_latency_weights) in peers_and_latencies_by_distance {
        // Sort the peers by latency weights
        peers_and_latency_weights.sort_by_key(|(_, latency_weight)| *latency_weight);

        // Add the peers to the sorted list (in sorted order)
        sorted_peers.extend(
            peers_and_latency_weights
                .into_iter()
                .map(|(peer_network_id, _)| peer_network_id),
        );
    }

    sorted_peers
}

/// Converts the given latency measurement to a weight.
/// The lower the latency, the higher the weight.
fn convert_latency_to_weight(latency: Option<f64>) -> f64 {
    match latency {
        Some(latency) => {
            // If the latency is <= 0, something has gone wrong, so return 0.
            if latency <= 0.0 {
                return 0.0;
            }

            // Otherwise, invert the latency to get the weight
            1000.0 / latency
        },
        None => 0.0, // If the latency is missing, return 0.0
    }
}

/// Gets the distance from the validators for the specified peer from the peer metadata
fn get_distance_for_peer(
    peer_network_id: &PeerNetworkId,
    peer_metadata: &PeerMetadata,
) -> Option<u64> {
    // Get the distance for the peer
    let peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata();
    let distance = peer_monitoring_metadata
        .latest_network_info_response
        .as_ref()
        .map(|response| response.distance_from_validators);

    // If the distance is missing, log a warning
    if distance.is_none() {
        warn!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Unable to get distance for peer! Peer: {:?}",
                peer_network_id
            ))
        );
    }

    distance
}

/// Gets the latency for the specified peer from the peer metadata
fn get_latency_for_peer(
    peer_network_id: &PeerNetworkId,
    peer_metadata: &PeerMetadata,
) -> Option<f64> {
    // Get the latency for the peer
    let peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata();
    let latency = peer_monitoring_metadata.average_ping_latency_secs;

    // If the latency is missing, log a warning
    if latency.is_none() {
        warn!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Unable to get latency for peer! Peer: {:?}",
                peer_network_id
            ))
        );
    }

    latency
}
