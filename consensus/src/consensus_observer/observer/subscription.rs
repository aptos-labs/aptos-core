// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::common::{
    error::Error,
    logging::{LogEntry, LogSchema},
};
use aptos_config::{config::ConsensusObserverConfig, network_id::PeerNetworkId};
use aptos_logger::{info, warn};
use aptos_network::{application::metadata::PeerMetadata, ProtocolId};
use aptos_storage_interface::DbReader;
use aptos_time_service::{TimeService, TimeServiceTrait};
use ordered_float::OrderedFloat;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
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

    // The timestamp of the last message received for the subscription
    last_message_receive_time: Instant,

    // The timestamp and connected peers for the last optimality check
    last_optimality_check_time_and_peers: (Instant, HashSet<PeerNetworkId>),

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
        // Get the current time
        let time_now = time_service.now();

        // Create a new subscription
        Self {
            consensus_observer_config,
            db_reader,
            peer_network_id,
            last_message_receive_time: time_now,
            last_optimality_check_time_and_peers: (time_now, HashSet::new()),
            highest_synced_version_and_time: (0, time_now),
            time_service,
        }
    }

    /// Verifies that the peer currently selected for the subscription is
    /// optimal. This is only done if: (i) the peers have changed since the
    /// last check; or (ii) enough time has elapsed to force a refresh.
    pub fn check_subscription_peer_optimality(
        &mut self,
        peers_and_metadata: &HashMap<PeerNetworkId, PeerMetadata>,
    ) -> Result<(), Error> {
        // Get the last optimality check time and connected peers
        let (last_optimality_check_time, last_optimality_check_peers) =
            self.last_optimality_check_time_and_peers.clone();

        // Determine if enough time has elapsed to force a refresh
        let time_now = self.time_service.now();
        let duration_since_last_check = time_now.duration_since(last_optimality_check_time);
        let refresh_interval = Duration::from_millis(
            self.consensus_observer_config
                .subscription_refresh_interval_ms,
        );
        let force_refresh = duration_since_last_check >= refresh_interval;

        // Determine if the peers have changed since the last check.
        // Note: we only check for peer changes periodically to avoid
        // excessive subscription churn due to peer connects/disconnects.
        let current_connected_peers = peers_and_metadata.keys().cloned().collect();
        let peer_check_interval = Duration::from_millis(
            self.consensus_observer_config
                .subscription_peer_change_interval_ms,
        );
        let peers_changed = duration_since_last_check >= peer_check_interval
            && current_connected_peers != last_optimality_check_peers;

        // Determine if we should perform the optimality check
        if !force_refresh && !peers_changed {
            return Ok(()); // We don't need to check optimality yet
        }

        // Otherwise, update the last peer optimality check time and peers
        self.last_optimality_check_time_and_peers = (time_now, current_connected_peers);

        // Sort the peers by subscription optimality
        let sorted_peers = sort_peers_by_subscription_optimality(peers_and_metadata);

        // Verify that this peer is one of the most optimal peers
        let max_concurrent_subscriptions =
            self.consensus_observer_config.max_concurrent_subscriptions as usize;
        if !sorted_peers
            .iter()
            .take(max_concurrent_subscriptions)
            .any(|peer| peer == &self.peer_network_id)
        {
            return Err(Error::SubscriptionSuboptimal(format!(
                "Subscription to peer: {} is no longer optimal! New optimal peers: {:?}",
                self.peer_network_id, sorted_peers
            )));
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

    /// Updates the last message receive time to the current time
    pub fn update_last_message_receive_time(&mut self) {
        self.last_message_receive_time = self.time_service.now();
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

/// Sorts the peers by subscription optimality (in descending order of
/// optimality). This requires: (i) sorting the peers by distance from the
/// validator set and ping latency (lower values are more optimal); and (ii)
/// filtering out peers that don't support consensus observer.
///
/// Note: we prioritize distance over latency as we want to avoid close
/// but not up-to-date peers. If peers don't have sufficient metadata
/// for sorting, they are given a lower priority.
pub fn sort_peers_by_subscription_optimality(
    peers_and_metadata: &HashMap<PeerNetworkId, PeerMetadata>,
) -> Vec<PeerNetworkId> {
    // Group peers and latencies by validator distance, i.e., distance -> [(peer, latency)]
    let mut unsupported_peers = Vec::new();
    let mut peers_and_latencies_by_distance = BTreeMap::new();
    for (peer_network_id, peer_metadata) in peers_and_metadata {
        // Verify that the peer supports consensus observer
        if !supports_consensus_observer(peer_metadata) {
            unsupported_peers.push(*peer_network_id);
            continue; // Skip the peer
        }

        // Get the distance and latency for the peer
        let distance = get_distance_for_peer(peer_network_id, peer_metadata);
        let latency = get_latency_for_peer(peer_network_id, peer_metadata);

        // If the distance is not found, use the maximum distance
        let distance =
            distance.unwrap_or(aptos_peer_monitoring_service_types::MAX_DISTANCE_FROM_VALIDATORS);

        // If the latency is not found, use a large latency
        let latency = latency.unwrap_or(MAX_PING_LATENCY_SECS);

        // Add the peer and latency to the distance group
        peers_and_latencies_by_distance
            .entry(distance)
            .or_insert_with(Vec::new)
            .push((*peer_network_id, OrderedFloat(latency)));
    }

    // If there are peers that don't support consensus observer, log them
    if !unsupported_peers.is_empty() {
        info!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Found {} peers that don't support consensus observer! Peers: {:?}",
                unsupported_peers.len(),
                unsupported_peers
            ))
        );
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

    // Log the sorted peers
    info!(
        LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
            "Sorted {} peers by subscription optimality! Peers: {:?}",
            sorted_peers.len(),
            sorted_peers
        ))
    );

    sorted_peers
}

/// Returns true iff the peer metadata indicates support for consensus observer
fn supports_consensus_observer(peer_metadata: &PeerMetadata) -> bool {
    peer_metadata.supports_protocol(ProtocolId::ConsensusObserver)
        && peer_metadata.supports_protocol(ProtocolId::ConsensusObserverRpc)
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_config::config::PeerRole;
    use aptos_netcore::transport::ConnectionOrigin;
    use aptos_network::{
        protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolIdSet},
        transport::{ConnectionId, ConnectionMetadata},
    };
    use aptos_peer_monitoring_service_types::{
        response::NetworkInformationResponse, PeerMonitoringMetadata,
    };
    use aptos_storage_interface::Result;
    use aptos_types::{network_address::NetworkAddress, transaction::Version};
    use claims::assert_matches;
    use mockall::mock;

    // This is a simple mock of the DbReader (it generates a MockDatabaseReader)
    mock! {
        pub DatabaseReader {}
        impl DbReader for DatabaseReader {
            fn get_latest_ledger_info_version(&self) -> Result<Version>;
        }
    }

    #[test]
    fn test_check_subscription_peer_optimality_single() {
        // Create a consensus observer config with a maximum of 1 subscription
        let consensus_observer_config = create_observer_config(1);

        // Create a new observer subscription
        let time_service = TimeService::mock();
        let peer_network_id = PeerNetworkId::random();
        let mut subscription = ConsensusObserverSubscription::new(
            consensus_observer_config,
            Arc::new(MockDatabaseReader::new()),
            peer_network_id,
            time_service.clone(),
        );

        // Verify the time and peers for the last optimality check
        let mock_time_service = time_service.into_mock();
        verify_last_check_time_and_peers(&subscription, mock_time_service.now(), HashSet::new());

        // Create a peers and metadata map for the subscription
        let mut peers_and_metadata = HashMap::new();
        add_metadata_for_peer(&mut peers_and_metadata, peer_network_id, true, false);

        // Add a more optimal peer to the set of peers
        let new_optimal_peer = PeerNetworkId::random();
        add_metadata_for_peer(&mut peers_and_metadata, new_optimal_peer, true, true);

        // Verify that the peer is optimal (not enough time has elapsed to check)
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, true);

        // Elapse some amount of time (but not enough to check optimality)
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_peer_change_interval_ms / 2,
        ));

        // Verify that the peer is still optimal (not enough time has elapsed to check)
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, true);

        // Elapse enough time to check the peer optimality
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_peer_change_interval_ms + 1,
        ));

        // Verify that the peer is no longer optimal (a more optimal peer has been added)
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, false);

        // Verify the time of the last peer optimality check
        verify_last_check_time_and_peers(
            &subscription,
            mock_time_service.now(),
            peers_and_metadata.keys().cloned().collect(),
        );

        // Elapse enough time to check the peer optimality
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_peer_change_interval_ms + 1,
        ));

        // Verify that the peer is now optimal (the peers haven't changed)
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, true);

        // Remove the current peer from the list of peers
        peers_and_metadata.remove(&peer_network_id);

        // Verify that the peer is not optimal (the peers have changed)
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, false);

        // Verify the time of the last peer optimality check
        verify_last_check_time_and_peers(
            &subscription,
            mock_time_service.now(),
            peers_and_metadata.keys().cloned().collect(),
        );
    }

    #[test]
    fn test_check_subscription_peer_optimality_multiple() {
        // Create a consensus observer config with a maximum of 2 subscriptions
        let consensus_observer_config = create_observer_config(2);

        // Create a new observer subscription
        let time_service = TimeService::mock();
        let peer_network_id = PeerNetworkId::random();
        let mut subscription = ConsensusObserverSubscription::new(
            consensus_observer_config,
            Arc::new(MockDatabaseReader::new()),
            peer_network_id,
            time_service.clone(),
        );

        // Create a peers and metadata map for the subscription
        let mut peers_and_metadata = HashMap::new();
        add_metadata_for_peer(&mut peers_and_metadata, peer_network_id, true, false);

        // Add a more optimal peer to the set of peers
        let new_optimal_peer = PeerNetworkId::random();
        add_metadata_for_peer(&mut peers_and_metadata, new_optimal_peer, true, true);

        // Elapse enough time to check the peer optimality
        let mock_time_service = time_service.into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_peer_change_interval_ms + 1,
        ));

        // Verify that the peer is optimal (it's in the top 2 most optimal peers)
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, true);

        // Add another more optimal peer to the set of peers
        let another_optimal_peer = PeerNetworkId::random();
        add_metadata_for_peer(&mut peers_and_metadata, another_optimal_peer, true, true);

        // Elapse enough time to check the peer optimality
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_peer_change_interval_ms + 1,
        ));

        // Verify that the peer is no longer optimal (it's not in the top 2 most optimal peers)
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, false);

        // Remove the previous optimal peer from the list of peers
        peers_and_metadata.remove(&new_optimal_peer);

        // Elapse enough time to check the peer optimality
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_peer_change_interval_ms + 1,
        ));

        // Verify that the peer is optimal (it's in the top 2 most optimal peers)
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, true);
    }

    #[test]
    fn test_check_subscription_peer_refresh() {
        // Create a consensus observer config with a maximum of 1 subscription
        let consensus_observer_config = create_observer_config(1);

        // Create a new observer subscription
        let time_service = TimeService::mock();
        let peer_network_id = PeerNetworkId::random();
        let mut subscription = ConsensusObserverSubscription::new(
            consensus_observer_config,
            Arc::new(MockDatabaseReader::new()),
            peer_network_id,
            time_service.clone(),
        );

        // Create a peers and metadata map for the subscription
        let mut peers_and_metadata = HashMap::new();
        add_metadata_for_peer(&mut peers_and_metadata, peer_network_id, true, false);

        // Verify that the peer is optimal (not enough time has elapsed to refresh)
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, true);

        // Add a more optimal peer to the set of peers
        let new_optimal_peer = PeerNetworkId::random();
        add_metadata_for_peer(&mut peers_and_metadata, new_optimal_peer, true, true);

        // Verify that the peer is still optimal (not enough time has elapsed to refresh)
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, true);

        // Elapse enough time to refresh optimality
        let mock_time_service = time_service.into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_refresh_interval_ms + 1,
        ));

        // Verify that the peer is no longer optimal
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, false);

        // Elapse some amount of time (but not enough to refresh)
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_refresh_interval_ms / 2,
        ));

        // Verify that the peer is now optimal (not enough time has elapsed to refresh)
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, true);

        // Remove the more optimal peer from the list of peers
        peers_and_metadata.remove(&new_optimal_peer);

        // Elapse enough time to refresh optimality
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_refresh_interval_ms + 1,
        ));

        // Verify that the peer is optimal
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, true);

        // Verify the time of the last peer optimality check
        verify_last_check_time_and_peers(
            &subscription,
            mock_time_service.now(),
            peers_and_metadata.keys().cloned().collect(),
        );
    }

    #[test]
    fn test_check_subscription_peer_optimality_supported() {
        // Create a consensus observer config with a maximum of 1 subscription
        let consensus_observer_config = create_observer_config(1);

        // Create a new observer subscription
        let time_service = TimeService::mock();
        let peer_network_id = PeerNetworkId::random();
        let mut subscription = ConsensusObserverSubscription::new(
            consensus_observer_config,
            Arc::new(MockDatabaseReader::new()),
            peer_network_id,
            time_service.clone(),
        );

        // Insert empty metadata for the subscription peer
        let mut peers_and_metadata = HashMap::new();
        add_metadata_for_peer(&mut peers_and_metadata, peer_network_id, true, false);

        // Elapse enough time to check optimality
        let mock_time_service = time_service.into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_peer_change_interval_ms + 1,
        ));

        // Verify that the peer is still optimal (there are no other peers)
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, true);

        // Add a more optimal peer without consensus observer support
        let unsupported_peer = PeerNetworkId::random();
        add_metadata_for_peer(&mut peers_and_metadata, unsupported_peer, false, false);

        // Elapse enough time to check optimality
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_peer_change_interval_ms + 1,
        ));

        // Verify that the peer is still optimal (the unsupported peer is ignored)
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, true);

        // Add another more optimal peer with consensus observer support
        let supported_peer = PeerNetworkId::random();
        add_metadata_for_peer(&mut peers_and_metadata, supported_peer, true, true);

        // Elapse enough time to check optimality
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_peer_change_interval_ms + 1,
        ));

        // Verify that the peer is no longer optimal
        verify_subscription_peer_optimality(&mut subscription, &peers_and_metadata, false);
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
        verify_subscription_time_out(&subscription, false);
        assert_eq!(subscription.last_message_receive_time, current_time);

        // Elapse some amount of time (but not enough to timeout)
        let mock_time_service = time_service.into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_subscription_timeout_ms / 2,
        ));

        // Verify that the subscription has not timed out
        verify_subscription_time_out(&subscription, false);

        // Update the last message receive time
        let current_time = mock_time_service.now();
        subscription.update_last_message_receive_time();
        assert_eq!(subscription.last_message_receive_time, current_time);

        // Verify that the subscription has not timed out
        verify_subscription_time_out(&subscription, false);

        // Elapse enough time to timeout the subscription
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_subscription_timeout_ms + 1,
        ));

        // Verify that the subscription has timed out
        verify_subscription_time_out(&subscription, true);
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
        let mock_time_service = time_service.into_mock();
        verify_subscription_syncing_progress(
            &mut subscription,
            first_synced_version,
            mock_time_service.now(),
        );

        // Elapse some amount of time (not enough to timeout)
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_synced_version_timeout_ms / 2,
        ));

        // Verify that the DB is still making sync progress
        verify_subscription_syncing_progress(
            &mut subscription,
            first_synced_version,
            mock_time_service.now(),
        );

        // Elapse enough time to timeout the subscription
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_synced_version_timeout_ms + 1,
        ));

        // Verify that the DB is still making sync progress (the next version is higher)
        verify_subscription_syncing_progress(
            &mut subscription,
            second_synced_version,
            mock_time_service.now(),
        );

        // Elapse enough time to timeout the subscription
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_synced_version_timeout_ms + 1,
        ));

        // Verify that the DB is not making sync progress and that the subscription has timed out
        assert_matches!(
            subscription.check_syncing_progress(),
            Err(Error::SubscriptionProgressStopped(_))
        );
    }

    #[test]
    fn test_update_last_message_receive_time() {
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

        // Verify the initial last message time
        assert_eq!(subscription.last_message_receive_time, time_service.now());

        // Elapse some amount of time
        let mock_time_service = time_service.into_mock();
        mock_time_service.advance(Duration::from_secs(10));

        // Update the last message time
        let current_time = mock_time_service.now();
        subscription.update_last_message_receive_time();

        // Verify that the last message time is updated
        assert_eq!(subscription.last_message_receive_time, current_time);
    }

    #[test]
    fn test_sort_peers_by_distance_and_latency() {
        // Sort an empty list of peers
        let peers_and_metadata = HashMap::new();
        assert!(sort_peers_by_subscription_optimality(&peers_and_metadata).is_empty());

        // Create a list of peers with empty metadata
        let peers_and_metadata = create_peers_and_metadata(true, true, true, 10);

        // Sort the peers and verify the results
        let sorted_peers = sort_peers_by_subscription_optimality(&peers_and_metadata);
        assert_eq!(sorted_peers.len(), 10);

        // Create a list of peers with valid metadata
        let peers_and_metadata = create_peers_and_metadata(false, false, true, 10);

        // Sort the peers
        let sorted_peers = sort_peers_by_subscription_optimality(&peers_and_metadata);

        // Verify the order of the peers
        verify_increasing_distance_latencies(&peers_and_metadata, &sorted_peers);
        assert_eq!(sorted_peers.len(), 10);

        // Create a list of peers with and without metadata
        let mut peers_and_metadata = create_peers_and_metadata(false, false, true, 10);
        peers_and_metadata.extend(create_peers_and_metadata(true, false, true, 10));
        peers_and_metadata.extend(create_peers_and_metadata(false, true, true, 10));
        peers_and_metadata.extend(create_peers_and_metadata(true, true, true, 10));

        // Sort the peers
        let sorted_peers = sort_peers_by_subscription_optimality(&peers_and_metadata);
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

    #[test]
    fn test_sort_peers_by_distance_and_latency_filter() {
        // Sort an empty list of peers
        let peers_and_metadata = HashMap::new();
        assert!(sort_peers_by_subscription_optimality(&peers_and_metadata).is_empty());

        // Create a list of peers with empty metadata (with consensus observer support)
        let peers_and_metadata = create_peers_and_metadata(true, true, true, 10);

        // Sort the peers and verify the results
        let sorted_peers = sort_peers_by_subscription_optimality(&peers_and_metadata);
        assert_eq!(sorted_peers.len(), 10);

        // Create a list of peers with empty metadata (without consensus observer support)
        let peers_and_metadata = create_peers_and_metadata(true, true, false, 10);

        // Sort the peers and verify the results
        let sorted_peers = sort_peers_by_subscription_optimality(&peers_and_metadata);
        assert!(sorted_peers.is_empty());

        // Create a list of peers with valid metadata (without consensus observer support)
        let peers_and_metadata = create_peers_and_metadata(false, false, false, 10);

        // Sort the peers and verify the results
        let sorted_peers = sort_peers_by_subscription_optimality(&peers_and_metadata);
        assert!(sorted_peers.is_empty());

        // Create a list of peers with empty metadata (with and without consensus observer support)
        let mut peers_and_metadata = create_peers_and_metadata(true, true, true, 5);
        peers_and_metadata.extend(create_peers_and_metadata(true, true, false, 50));

        // Sort the peers and verify the results (only the supported peers are sorted)
        let sorted_peers = sort_peers_by_subscription_optimality(&peers_and_metadata);
        assert_eq!(sorted_peers.len(), 5);

        // Create a list of peers with valid metadata (with and without consensus observer support)
        let mut peers_and_metadata = create_peers_and_metadata(false, false, true, 50);
        peers_and_metadata.extend(create_peers_and_metadata(false, false, false, 10));

        // Sort the peers and verify the results (only the supported peers are sorted)
        let sorted_peers = sort_peers_by_subscription_optimality(&peers_and_metadata);
        assert_eq!(sorted_peers.len(), 50);

        // Create a list of peers with valid metadata (with and without consensus observer support)
        let supported_peer_and_metadata = create_peers_and_metadata(false, false, true, 1);
        let unsupported_peer_and_metadata = create_peers_and_metadata(false, false, false, 1);
        let mut peers_and_metadata = HashMap::new();
        peers_and_metadata.extend(supported_peer_and_metadata.clone());
        peers_and_metadata.extend(unsupported_peer_and_metadata);

        // Sort the peers and verify the results (only the supported peer is sorted)
        let supported_peer = supported_peer_and_metadata.keys().next().unwrap();
        let sorted_peers = sort_peers_by_subscription_optimality(&peers_and_metadata);
        assert_eq!(sorted_peers, vec![*supported_peer]);
    }

    /// Adds metadata for the specified peer to the map of peers and metadata
    fn add_metadata_for_peer(
        peers_and_metadata: &mut HashMap<PeerNetworkId, PeerMetadata>,
        peer_network_id: PeerNetworkId,
        support_consensus_observer: bool,
        set_ping_latency: bool,
    ) {
        // Determine the ping latency to use for the peer
        let average_ping_latency = if set_ping_latency { Some(0.1) } else { None };

        // Add the peer and metadata to the map
        peers_and_metadata.insert(
            peer_network_id,
            PeerMetadata::new_for_test(
                create_connection_metadata(peer_network_id, support_consensus_observer),
                PeerMonitoringMetadata::new(average_ping_latency, None, None, None, None),
            ),
        );
    }

    /// Creates a new connection metadata for testing
    fn create_connection_metadata(
        peer_network_id: PeerNetworkId,
        support_consensus_observer: bool,
    ) -> ConnectionMetadata {
        if support_consensus_observer {
            // Create a protocol set that supports consensus observer
            let protocol_set = ProtocolIdSet::from_iter(vec![
                ProtocolId::ConsensusObserver,
                ProtocolId::ConsensusObserverRpc,
            ]);

            // Create the connection metadata with the protocol set
            ConnectionMetadata::new(
                peer_network_id.peer_id(),
                ConnectionId::default(),
                NetworkAddress::mock(),
                ConnectionOrigin::Inbound,
                MessagingProtocolVersion::V1,
                protocol_set,
                PeerRole::PreferredUpstream,
            )
        } else {
            ConnectionMetadata::mock(peer_network_id.peer_id())
        }
    }

    /// Creates a consensus observer config with the given max concurrent subscriptions
    fn create_observer_config(max_concurrent_subscriptions: u64) -> ConsensusObserverConfig {
        ConsensusObserverConfig {
            max_concurrent_subscriptions,
            ..ConsensusObserverConfig::default()
        }
    }

    /// Creates a new peer and metadata for testing
    fn create_peer_and_metadata(
        latency: Option<f64>,
        distance_from_validators: Option<u64>,
        support_consensus_observer: bool,
    ) -> (PeerNetworkId, PeerMetadata) {
        // Create a random peer
        let peer_network_id = PeerNetworkId::random();

        // Create a new peer metadata with the given latency and distance
        let connection_metadata =
            create_connection_metadata(peer_network_id, support_consensus_observer);
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
        support_consensus_observer: bool,
        num_peers: u64,
    ) -> HashMap<PeerNetworkId, PeerMetadata> {
        let mut peers_and_metadata = HashMap::new();
        for i in 1..num_peers + 1 {
            // Determine the distance for the peer
            let distance = if empty_distance { None } else { Some(i) };

            // Determine the latency for the peer
            let latency = if empty_latency { None } else { Some(i as f64) };

            // Create a new peer and metadata
            let (peer_network_id, peer_metadata) =
                create_peer_and_metadata(latency, distance, support_consensus_observer);
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

    /// Verifies that the last check time and peers are as expected
    fn verify_last_check_time_and_peers(
        subscription: &ConsensusObserverSubscription,
        expected_last_check_time: Instant,
        expected_last_check_peers: HashSet<PeerNetworkId>,
    ) {
        // Get the last check time and peers from the subscription
        let (last_check_time, last_check_peers) =
            subscription.last_optimality_check_time_and_peers.clone();

        // Verify the last check time and peers match the expected values
        assert_eq!(last_check_time, expected_last_check_time);
        assert_eq!(last_check_peers, expected_last_check_peers);
    }

    /// Verifies that the subscription time out matches the expected value
    fn verify_subscription_time_out(subscription: &ConsensusObserverSubscription, timed_out: bool) {
        // Check if the subscription has timed out
        let result = subscription.check_subscription_timeout();

        // Verify the result
        if timed_out {
            assert_matches!(result, Err(Error::SubscriptionTimeout(_)));
        } else {
            assert!(result.is_ok());
        }
    }

    /// Verifies that the peer optimality matches the expected value
    fn verify_subscription_peer_optimality(
        subscription: &mut ConsensusObserverSubscription,
        peers_and_metadata: &HashMap<PeerNetworkId, PeerMetadata>,
        is_optimal: bool,
    ) {
        // Check the subscription peer optimality
        let result = subscription.check_subscription_peer_optimality(peers_and_metadata);

        // Verify the result
        if is_optimal {
            assert!(result.is_ok());
        } else {
            assert_matches!(result, Err(Error::SubscriptionSuboptimal(_)));
        }
    }

    /// Verifies that the syncing progress is as expected
    fn verify_subscription_syncing_progress(
        subscription: &mut ConsensusObserverSubscription,
        first_synced_version: Version,
        time: Instant,
    ) {
        assert!(subscription.check_syncing_progress().is_ok());
        assert_eq!(
            subscription.highest_synced_version_and_time,
            (first_synced_version, time)
        );
    }
}
