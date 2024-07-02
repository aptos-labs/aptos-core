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

// A useful constant for representing the maximum ping latency
const MAX_PING_LATENCY_SECS: f64 = 10_000.0;

/// A single consensus observer subscription
pub struct ConsensusObserverSubscription {
    // The configuration of the consensus observer
    consensus_observer_config: ConsensusObserverConfig,

    // A handle to storage (used to read the latest state and check progress)
    db_reader: Arc<dyn DbReader>,

    // The peer network id of the active subscription
    peer_network_id: PeerNetworkId,

    // The timestamp of the last message received from the peer
    last_message_receive_time: Instant,

    // The timestamp of the last peer optimality check
    last_peer_optimality_check: Instant,

    // The highest synced version we've seen from storage, along with the time at which it was seen
    highest_synced_version_and_time: (u64, Instant),

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
            last_peer_optimality_check: time_now,
            highest_synced_version_and_time: (0, time_now),
            time_service,
        }
    }

    /// Verifies that the peer selected for the subscription is optimal
    /// based on the set of currently available peers. This is done
    /// periodically to avoid excessive subscription terminations.
    pub fn check_subscription_peer_optimality(
        &mut self,
        peers_and_metadata: HashMap<PeerNetworkId, PeerMetadata>,
    ) -> Result<(), Error> {
        // Check if we need to perform the peer optimality check
        let time_now = self.time_service.now();
        let duration_since_last_check = time_now.duration_since(self.last_peer_optimality_check);
        if duration_since_last_check
            < Duration::from_millis(
                self.consensus_observer_config
                    .peer_optimality_check_interval_ms,
            )
        {
            return Ok(()); // We don't need to check the peer optimality yet
        }

        // Update the last peer optimality check time
        self.last_peer_optimality_check = time_now;

        // Verify that we're subscribed to the most optimal peer
        if let Some(optimal_peer) = sort_peers_by_distance_and_latency(peers_and_metadata).first() {
            if *optimal_peer != self.peer_network_id {
                return Err(Error::SubscriptionSuboptimal(format!(
                    "Subscription to peer: {} is no longer optimal! New optimal peer: {}",
                    self.peer_network_id, optimal_peer
                )));
            }
        }

        Ok(())
    }

    /// Verifies that the subscription has not timed out based
    /// on the last received message time.
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
        let (highest_synced_version, highest_version_timestamp) =
            self.highest_synced_version_and_time;
        if current_synced_version <= highest_synced_version {
            // The synced version hasn't increased. Check if we should terminate
            // the subscription based on the last time the highest synced version was seen.
            let time_now = self.time_service.now();
            let duration_since_highest_seen = time_now.duration_since(highest_version_timestamp);
            if duration_since_highest_seen
                > Duration::from_millis(
                    self.consensus_observer_config.max_synced_version_timeout_ms,
                )
            {
                return Err(Error::SubscriptionProgressStopped(format!(
                    "The DB is not making sync progress! Highest synced version: {}, elapsed: {:?}",
                    highest_synced_version, duration_since_highest_seen
                )));
            }
        }

        // Update the highest synced version and time
        self.highest_synced_version_and_time = (current_synced_version, self.time_service.now());

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

/// Sorts the peers by distance from the validator set and latency.
/// We prioritize distance over latency as we want to avoid close
/// but not up-to-date peers. If peers don't have sufficient metadata
/// for sorting, they are given a lower priority.
pub fn sort_peers_by_distance_and_latency(
    peers_and_metadata: HashMap<PeerNetworkId, PeerMetadata>,
) -> Vec<PeerNetworkId> {
    // Group peers and latencies by validator distance, i.e., distance -> [(peer, latency)]
    let mut peers_and_latencies_by_distance = BTreeMap::new();
    for (peer_network_id, peer_metadata) in peers_and_metadata {
        // Get the distance and latency for the peer
        let distance = get_distance_for_peer(&peer_network_id, &peer_metadata);
        let latency = get_latency_for_peer(&peer_network_id, &peer_metadata);

        // If the distance is not found, use the maximum distance
        let distance =
            distance.unwrap_or(aptos_peer_monitoring_service_types::MAX_DISTANCE_FROM_VALIDATORS);

        // If the latency is not found, use a large latency
        let latency = latency.unwrap_or(MAX_PING_LATENCY_SECS);

        // Add the peer and latency to the distance group
        peers_and_latencies_by_distance
            .entry(distance)
            .or_insert_with(Vec::new)
            .push((peer_network_id, OrderedFloat(latency)));
    }

    // Sort the peers by distance and latency. Note: BTreeMaps are
    // sorted by key, so the entries will be sorted by distance in ascending order.
    let mut sorted_peers = Vec::new();
    for (_, mut peers_and_latencies) in peers_and_latencies_by_distance {
        // Sort the peers by latency
        peers_and_latencies.sort_by_key(|(_, latency)| *latency);

        // Add the peers to the sorted list (in sorted order)
        sorted_peers.extend(
            peers_and_latencies
                .into_iter()
                .map(|(peer_network_id, _)| peer_network_id),
        );
    }

    sorted_peers
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_network::transport::ConnectionMetadata;
    use aptos_peer_monitoring_service_types::{
        response::NetworkInformationResponse, PeerMonitoringMetadata,
    };
    use aptos_storage_interface::Result;
    use aptos_types::transaction::Version;
    use mockall::mock;

    // This is a simple mock of the DbReader (it generates a MockDatabaseReader)
    mock! {
    pub DatabaseReader {}
    impl DbReader for DatabaseReader {
            fn get_latest_ledger_info_version(&self) -> Result<Version>;
        }
    }

    #[test]
    fn check_subscription_peer_optimality() {
        // Create a new observer subscription
        let consensus_observer_config = ConsensusObserverConfig::default();
        let peer_network_id = PeerNetworkId::random();
        let time_service = TimeService::mock();
        let mut subscription = ConsensusObserverSubscription::new(
            consensus_observer_config,
            Arc::new(MockDatabaseReader::new()),
            peer_network_id,
            time_service.clone(),
        );

        // Verify the time of the last peer optimality check
        let current_time = time_service.now();
        assert_eq!(subscription.last_peer_optimality_check, current_time);

        // Verify that the peer is optimal (not enough time has elapsed to check)
        assert!(subscription
            .check_subscription_peer_optimality(HashMap::new())
            .is_ok());

        // Elapse some amount of time (but not enough to check optimality)
        let mock_time_service = time_service.into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.peer_optimality_check_interval_ms / 2,
        ));

        // Verify that the original peer is still optimal even though it is missing metadata
        let new_optimal_peer = PeerNetworkId::random();
        let mut peers_and_metadata = HashMap::new();
        peers_and_metadata.insert(
            new_optimal_peer,
            PeerMetadata::new_for_test(
                ConnectionMetadata::mock(new_optimal_peer.peer_id()),
                PeerMonitoringMetadata::new(None, None, None, None, None),
            ),
        );
        assert!(subscription
            .check_subscription_peer_optimality(peers_and_metadata.clone())
            .is_ok());

        // Elapse enough time to check optimality
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.peer_optimality_check_interval_ms + 1,
        ));

        // Verify that the original peer is no longer optimal
        assert!(subscription
            .check_subscription_peer_optimality(peers_and_metadata.clone())
            .is_err());

        // Add the original peer to the list of peers (with optimal metadata)
        peers_and_metadata.insert(
            peer_network_id,
            PeerMetadata::new_for_test(
                ConnectionMetadata::mock(peer_network_id.peer_id()),
                PeerMonitoringMetadata::new(Some(0.1), None, None, None, None),
            ),
        );

        // Verify that the peer is still optimal
        assert!(subscription
            .check_subscription_peer_optimality(peers_and_metadata)
            .is_ok());

        // Verify the time of the last peer optimality check
        let current_time = mock_time_service.now();
        assert_eq!(subscription.last_peer_optimality_check, current_time);
    }

    #[test]
    fn test_check_subscription_timeout() {
        // Create a new observer subscription
        let consensus_observer_config = ConsensusObserverConfig::default();
        let peer_network_id = PeerNetworkId::random();
        let time_service = TimeService::mock();
        let mut subscription = ConsensusObserverSubscription::new(
            consensus_observer_config,
            Arc::new(MockDatabaseReader::new()),
            peer_network_id,
            time_service.clone(),
        );

        // Verify that the subscription has not timed out and that the last message time is updated
        let current_time = time_service.now();
        assert!(subscription.check_subscription_timeout().is_ok());
        assert_eq!(subscription.last_message_receive_time, current_time);

        // Elapse some amount of time (but not enough to timeout)
        let mock_time_service = time_service.into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_subscription_timeout_ms / 2,
        ));

        // Verify that the subscription has not timed out
        assert!(subscription.check_subscription_timeout().is_ok());

        // Verify a new message is received successfully and that the last message time is updated
        let current_time = mock_time_service.now();
        subscription
            .verify_message_sender(&peer_network_id)
            .unwrap();
        assert_eq!(subscription.last_message_receive_time, current_time);

        // Verify that the subscription has not timed out
        assert!(subscription.check_subscription_timeout().is_ok());

        // Elapse enough time to timeout the subscription
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_subscription_timeout_ms + 1,
        ));

        // Verify that the subscription has timed out
        assert!(subscription.check_subscription_timeout().is_err());
    }

    #[test]
    fn test_check_syncing_progress() {
        // Create a mock DB reader with expectations
        let first_synced_version = 10;
        let second_synced_version = 20;
        let mut mock_db_reader = MockDatabaseReader::new();
        mock_db_reader
            .expect_get_latest_ledger_info_version()
            .returning(move || Ok(first_synced_version))
            .times(2); // Only allow two calls for the first version
        mock_db_reader
            .expect_get_latest_ledger_info_version()
            .returning(move || Ok(second_synced_version)); // Allow multiple calls for the second version

        // Create a new observer subscription
        let consensus_observer_config = ConsensusObserverConfig::default();
        let peer_network_id = PeerNetworkId::random();
        let time_service = TimeService::mock();
        let mut subscription = ConsensusObserverSubscription::new(
            consensus_observer_config,
            Arc::new(mock_db_reader),
            peer_network_id,
            time_service.clone(),
        );

        // Verify that the DB is making sync progress and that the highest synced version is updated
        let current_time = time_service.now();
        assert!(subscription.check_syncing_progress().is_ok());
        assert_eq!(
            subscription.highest_synced_version_and_time,
            (first_synced_version, current_time)
        );

        // Elapse some amount of time (not enough to timeout)
        let mock_time_service = time_service.into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_synced_version_timeout_ms / 2,
        ));

        // Verify that the DB is still making sync progress
        let current_time = mock_time_service.now();
        assert!(subscription.check_syncing_progress().is_ok());
        assert_eq!(
            subscription.highest_synced_version_and_time,
            (first_synced_version, current_time)
        );

        // Elapse enough time to timeout the subscription
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_synced_version_timeout_ms + 1,
        ));

        // Verify that the DB is still making sync progress (the next version is higher)
        let current_time = mock_time_service.now();
        assert!(subscription.check_syncing_progress().is_ok());
        assert_eq!(
            subscription.highest_synced_version_and_time,
            (second_synced_version, current_time)
        );

        // Elapse enough time to timeout the subscription
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_synced_version_timeout_ms + 1,
        ));

        // Verify that the DB is not making sync progress and that the subscription has timed out
        assert!(subscription.check_syncing_progress().is_err());
    }

    #[test]
    fn test_verify_message_sender() {
        // Create a new observer subscription
        let consensus_observer_config = ConsensusObserverConfig::default();
        let peer_network_id = PeerNetworkId::random();
        let time_service = TimeService::mock();
        let mut subscription = ConsensusObserverSubscription::new(
            consensus_observer_config,
            Arc::new(MockDatabaseReader::new()),
            peer_network_id,
            time_service.clone(),
        );

        // Verify that the message sender is valid
        let current_time = time_service.now();
        assert!(subscription.verify_message_sender(&peer_network_id).is_ok());
        assert_eq!(subscription.last_message_receive_time, current_time);

        // Elapse some amount of time
        let mock_time_service = time_service.into_mock();
        mock_time_service.advance(Duration::from_secs(10));

        // Verify that the message sender is not the expected peer
        let other_peer_network_id = PeerNetworkId::random();
        assert!(subscription
            .verify_message_sender(&other_peer_network_id)
            .is_err());
        assert_eq!(subscription.last_message_receive_time, current_time);

        // Elapse more time
        mock_time_service.advance(Duration::from_secs(10));

        // Verify that the message sender is the expected peer and that the last message time is updated
        let current_time = mock_time_service.now();
        assert!(subscription.verify_message_sender(&peer_network_id).is_ok());
        assert_eq!(subscription.last_message_receive_time, current_time);
    }

    #[test]
    fn test_sort_peers_by_distance_and_latency() {
        // Sort an empty list of peers
        let peers_and_metadata = HashMap::new();
        assert!(sort_peers_by_distance_and_latency(peers_and_metadata).is_empty());

        // Create a list of peers with empty metadata
        let peers_and_metadata = create_peers_and_metadata(true, true, 10);

        // Sort the peers and verify the results
        let sorted_peers = sort_peers_by_distance_and_latency(peers_and_metadata);
        assert_eq!(sorted_peers.len(), 10);

        // Create a list of peers with valid metadata
        let peers_and_metadata = create_peers_and_metadata(false, false, 10);

        // Sort the peers
        let sorted_peers = sort_peers_by_distance_and_latency(peers_and_metadata.clone());

        // Verify the order of the peers
        verify_increasing_distance_latencies(&peers_and_metadata, &sorted_peers);
        assert_eq!(sorted_peers.len(), 10);

        // Create a list of peers with and without metadata
        let mut peers_and_metadata = create_peers_and_metadata(false, false, 10);
        peers_and_metadata.extend(create_peers_and_metadata(true, false, 10));
        peers_and_metadata.extend(create_peers_and_metadata(false, true, 10));
        peers_and_metadata.extend(create_peers_and_metadata(true, true, 10));

        // Sort the peers
        let sorted_peers = sort_peers_by_distance_and_latency(peers_and_metadata.clone());
        assert_eq!(sorted_peers.len(), 40);

        // Verify the order of the first 20 peers
        let (first_20_peers, sorted_peers) = sorted_peers.split_at(20);
        verify_increasing_distance_latencies(&peers_and_metadata, first_20_peers);

        // Verify that the next 10 peers only have latency metadata
        let (next_10_peers, sorted_peers) = sorted_peers.split_at(10);
        for sorted_peer in next_10_peers {
            let peer_metadata = peers_and_metadata.get(sorted_peer).unwrap();
            assert!(get_distance_for_peer(sorted_peer, peer_metadata).is_none());
            assert!(get_latency_for_peer(sorted_peer, peer_metadata).is_some());
        }

        // Verify that the last 10 peers have no metadata
        let (last_10_peers, remaining_peers) = sorted_peers.split_at(10);
        for sorted_peer in last_10_peers {
            let peer_metadata = peers_and_metadata.get(sorted_peer).unwrap();
            assert!(get_distance_for_peer(sorted_peer, peer_metadata).is_none());
            assert!(get_latency_for_peer(sorted_peer, peer_metadata).is_none());
        }
        assert!(remaining_peers.is_empty());
    }

    /// Creates a new peer and metadata for testing
    fn create_peer_and_metadata(
        latency: Option<f64>,
        distance_from_validators: Option<u64>,
    ) -> (PeerNetworkId, PeerMetadata) {
        // Create a random peer
        let peer_network_id = PeerNetworkId::random();

        // Create a new peer metadata with the given latency and distance
        let connection_metadata = ConnectionMetadata::mock(peer_network_id.peer_id());
        let network_information_response =
            distance_from_validators.map(|distance| NetworkInformationResponse {
                connected_peers: BTreeMap::new(),
                distance_from_validators: distance,
            });
        let peer_monitoring_metadata =
            PeerMonitoringMetadata::new(latency, None, network_information_response, None, None);
        let peer_metadata =
            PeerMetadata::new_for_test(connection_metadata, peer_monitoring_metadata);

        (peer_network_id, peer_metadata)
    }

    /// Creates a list of peers and metadata for testing
    fn create_peers_and_metadata(
        empty_latency: bool,
        empty_distance: bool,
        num_peers: u64,
    ) -> HashMap<PeerNetworkId, PeerMetadata> {
        let mut peers_and_metadata = HashMap::new();
        for i in 1..num_peers + 1 {
            // Determine the distance for the peer
            let distance = if empty_distance { None } else { Some(i) };

            // Determine the latency for the peer
            let latency = if empty_latency { None } else { Some(i as f64) };

            // Create a new peer and metadata
            let (peer_network_id, peer_metadata) = create_peer_and_metadata(latency, distance);
            peers_and_metadata.insert(peer_network_id, peer_metadata);
        }
        peers_and_metadata
    }

    /// Verifies that the distance and latencies for the peers are in
    /// increasing order (with the distance taking precedence over the latency).
    fn verify_increasing_distance_latencies(
        peers_and_metadata: &HashMap<PeerNetworkId, PeerMetadata>,
        sorted_peers: &[PeerNetworkId],
    ) {
        let mut previous_latency = None;
        let mut previous_distance = 0;
        for sorted_peer in sorted_peers {
            // Get the distance and latency for the peer
            let peer_metadata = peers_and_metadata.get(sorted_peer).unwrap();
            let distance = get_distance_for_peer(sorted_peer, peer_metadata).unwrap();
            let latency = get_latency_for_peer(sorted_peer, peer_metadata);

            // Verify the order of the peers
            if distance == previous_distance {
                if let Some(latency) = latency {
                    if let Some(previous_latency) = previous_latency {
                        assert!(latency >= previous_latency);
                    }
                }
            } else {
                assert!(distance > previous_distance);
            }

            // Update the previous latency and distance
            previous_latency = latency;
            previous_distance = distance;
        }
    }
}
