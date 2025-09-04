// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{common::error::Error, observer::subscription_utils};
use velor_config::{config::ConsensusObserverConfig, network_id::PeerNetworkId};
use velor_network::application::metadata::PeerMetadata;
use velor_storage_interface::DbReader;
use velor_time_service::{TimeService, TimeServiceTrait};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

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

    /// Checks if the subscription is still healthy. If not, an error
    /// is returned indicating the reason for the subscription failure.
    pub fn check_subscription_health(
        &mut self,
        connected_peers_and_metadata: &HashMap<PeerNetworkId, PeerMetadata>,
        skip_peer_optimality_check: bool,
    ) -> Result<(), Error> {
        // Verify the subscription peer is still connected
        let peer_network_id = self.get_peer_network_id();
        if !connected_peers_and_metadata.contains_key(&peer_network_id) {
            return Err(Error::SubscriptionDisconnected(format!(
                "The peer: {:?} is no longer connected!",
                peer_network_id
            )));
        }

        // Verify the subscription has not timed out
        self.check_subscription_timeout()?;

        // Verify that the DB is continuing to sync and commit new data
        self.check_syncing_progress()?;

        // Verify that the subscription peer is still optimal
        self.check_subscription_peer_optimality(
            connected_peers_and_metadata,
            skip_peer_optimality_check,
        )?;

        // The subscription seems healthy
        Ok(())
    }

    /// Verifies that the peer currently selected for the subscription is
    /// optimal. This is only done if: (i) the peers have changed since the
    /// last check; or (ii) enough time has elapsed to force a refresh.
    ///
    /// Note: if `skip_peer_optimality_check` is true, the optimality check
    /// is trivially skipped, and the last update time is refreshed. This is
    /// useful to minimize excessive churn in the subscription set.
    fn check_subscription_peer_optimality(
        &mut self,
        peers_and_metadata: &HashMap<PeerNetworkId, PeerMetadata>,
        skip_peer_optimality_check: bool,
    ) -> Result<(), Error> {
        // Get the last optimality check time and connected peers
        let (last_optimality_check_time, last_optimality_check_peers) =
            self.last_optimality_check_time_and_peers.clone();

        // If we're skipping the peer optimality check, update the last check time and return
        let time_now = self.time_service.now();
        if skip_peer_optimality_check {
            self.last_optimality_check_time_and_peers = (time_now, last_optimality_check_peers);
            return Ok(());
        }

        // Determine if enough time has elapsed to force a refresh
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
        let sorted_peers =
            subscription_utils::sort_peers_by_subscription_optimality(peers_and_metadata);

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
    fn check_subscription_timeout(&self) -> Result<(), Error> {
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
    fn check_syncing_progress(&mut self) -> Result<(), Error> {
        // Get the current time and synced version from storage
        let time_now = self.time_service.now();
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
            let duration_since_highest_seen = time_now.duration_since(highest_version_timestamp);
            let timeout_duration = Duration::from_millis(
                self.consensus_observer_config
                    .max_subscription_sync_timeout_ms,
            );
            if duration_since_highest_seen > timeout_duration {
                return Err(Error::SubscriptionProgressStopped(format!(
                    "The DB is not making sync progress! Highest synced version: {}, elapsed: {:?}",
                    highest_synced_version, duration_since_highest_seen
                )));
            }
            return Ok(()); // We haven't timed out yet
        }

        // Update the highest synced version and time
        self.highest_synced_version_and_time = (current_synced_version, time_now);

        Ok(())
    }

    /// Returns the peer network id of the subscription
    pub fn get_peer_network_id(&self) -> PeerNetworkId {
        self.peer_network_id
    }

    /// Updates the last message receive time to the current time
    pub fn update_last_message_receive_time(&mut self) {
        self.last_message_receive_time = self.time_service.now();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use velor_config::config::PeerRole;
    use velor_netcore::transport::ConnectionOrigin;
    use velor_network::{
        protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolIdSet},
        transport::{ConnectionId, ConnectionMetadata},
        ProtocolId,
    };
    use velor_peer_monitoring_service_types::PeerMonitoringMetadata;
    use velor_storage_interface::Result;
    use velor_types::{network_address::NetworkAddress, transaction::Version};
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
    fn test_check_subscription_health_connected_and_timeout() {
        // Create a consensus observer config
        let consensus_observer_config = ConsensusObserverConfig {
            max_subscription_sync_timeout_ms: 100_000_000, // Use a large value so that we don't get DB progress errors
            ..ConsensusObserverConfig::default()
        };

        // Create a new observer subscription
        let time_service = TimeService::mock();
        let peer_network_id = PeerNetworkId::random();
        let mut subscription = ConsensusObserverSubscription::new(
            consensus_observer_config,
            Arc::new(MockDatabaseReader::new()),
            peer_network_id,
            time_service.clone(),
        );

        // Verify that the subscription is unhealthy (the peer is not connected)
        assert_matches!(
            subscription.check_subscription_health(&HashMap::new(), false),
            Err(Error::SubscriptionDisconnected(_))
        );

        // Create a peers and metadata map for the subscription
        let mut peers_and_metadata = HashMap::new();
        add_metadata_for_peer(&mut peers_and_metadata, peer_network_id, true, false);

        // Elapse enough time to timeout the subscription
        let mock_time_service = time_service.into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_subscription_timeout_ms + 1,
        ));

        // Verify that the subscription has timed out
        assert_matches!(
            subscription.check_subscription_health(&peers_and_metadata, false),
            Err(Error::SubscriptionTimeout(_))
        );
    }

    #[test]
    fn test_check_subscription_health_progress() {
        // Create a consensus observer config with a large timeout
        let consensus_observer_config = ConsensusObserverConfig {
            max_subscription_timeout_ms: 100_000_000, // Use a large value so that we don't time out
            ..ConsensusObserverConfig::default()
        };

        // Create a mock DB reader with expectations
        let first_synced_version = 1;
        let second_synced_version = 2;
        let mut mock_db_reader = MockDatabaseReader::new();
        mock_db_reader
            .expect_get_latest_ledger_info_version()
            .returning(move || Ok(first_synced_version))
            .times(1); // Only allow one call for the first version
        mock_db_reader
            .expect_get_latest_ledger_info_version()
            .returning(move || Ok(second_synced_version)); // Allow multiple calls for the second version

        // Create a new observer subscription
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

        // Elapse enough time to timeout the subscription
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_subscription_sync_timeout_ms + 1,
        ));

        // Verify that the DB is still making sync progress (the next version is higher)
        verify_subscription_syncing_progress(
            &mut subscription,
            second_synced_version,
            mock_time_service.now(),
        );

        // Elapse enough time to timeout the subscription
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_subscription_sync_timeout_ms + 1,
        ));

        // Verify that the DB is not making sync progress and that the subscription has timed out
        assert_matches!(
            subscription.check_syncing_progress(),
            Err(Error::SubscriptionProgressStopped(_))
        );
    }

    #[test]
    fn test_check_subscription_health_optimality() {
        // Create a consensus observer config with a single subscription and large timeouts
        let consensus_observer_config = ConsensusObserverConfig {
            max_concurrent_subscriptions: 1,
            max_subscription_timeout_ms: 100_000_000, // Use a large value so that we don't time out
            max_subscription_sync_timeout_ms: 100_000_000, // Use a large value so that we don't get DB progress errors
            ..ConsensusObserverConfig::default()
        };

        // Create a mock DB reader with expectations
        let mut mock_db_reader = MockDatabaseReader::new();
        mock_db_reader
            .expect_get_latest_ledger_info_version()
            .returning(move || Ok(1));

        // Create a new observer subscription
        let time_service = TimeService::mock();
        let peer_network_id = PeerNetworkId::random();
        let mut subscription = ConsensusObserverSubscription::new(
            consensus_observer_config,
            Arc::new(mock_db_reader),
            peer_network_id,
            time_service.clone(),
        );

        // Create a peers and metadata map for the subscription
        let mut peers_and_metadata = HashMap::new();
        add_metadata_for_peer(&mut peers_and_metadata, peer_network_id, true, false);

        // Verify that the subscription is healthy
        assert!(subscription
            .check_subscription_health(&peers_and_metadata, false)
            .is_ok());

        // Add a more optimal peer to the set of peers
        let new_optimal_peer = PeerNetworkId::random();
        add_metadata_for_peer(&mut peers_and_metadata, new_optimal_peer, true, true);

        // Elapse enough time for a peer optimality check
        let mock_time_service = time_service.into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_peer_change_interval_ms + 1,
        ));

        // Verify that the subscription is no longer optimal
        assert_matches!(
            subscription.check_subscription_health(&peers_and_metadata, false),
            Err(Error::SubscriptionSuboptimal(_))
        );
    }

    #[test]
    fn test_check_subscription_health_optimality_skipped() {
        // Create a consensus observer config with a single subscription and large timeouts
        let consensus_observer_config = ConsensusObserverConfig {
            max_concurrent_subscriptions: 1,
            max_subscription_timeout_ms: 100_000_000, // Use a large value so that we don't time out
            max_subscription_sync_timeout_ms: 100_000_000, // Use a large value so that we don't get DB progress errors
            ..ConsensusObserverConfig::default()
        };

        // Create a mock DB reader with expectations
        let mut mock_db_reader = MockDatabaseReader::new();
        mock_db_reader
            .expect_get_latest_ledger_info_version()
            .returning(move || Ok(1));

        // Create a new observer subscription
        let time_service = TimeService::mock();
        let peer_network_id = PeerNetworkId::random();
        let mut subscription = ConsensusObserverSubscription::new(
            consensus_observer_config,
            Arc::new(mock_db_reader),
            peer_network_id,
            time_service.clone(),
        );

        // Create a peers and metadata map for the subscription
        let mut peers_and_metadata = HashMap::new();
        add_metadata_for_peer(&mut peers_and_metadata, peer_network_id, true, false);

        // Verify that the subscription is healthy
        assert!(subscription
            .check_subscription_health(&peers_and_metadata, false)
            .is_ok());

        // Add a more optimal peer to the set of peers
        let new_optimal_peer = PeerNetworkId::random();
        add_metadata_for_peer(&mut peers_and_metadata, new_optimal_peer, true, true);

        // Elapse enough time for a peer optimality check
        let mock_time_service = time_service.into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_peer_change_interval_ms + 1,
        ));

        // Verify that the subscription is no longer optimal
        assert_matches!(
            subscription.check_subscription_health(&peers_and_metadata, false),
            Err(Error::SubscriptionSuboptimal(_))
        );

        // Elapse more time for a peer optimality check
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_peer_change_interval_ms + 1,
        ));

        // Add a less optimal peer to the set of peers
        let less_optimal_peer = PeerNetworkId::random();
        add_metadata_for_peer(&mut peers_and_metadata, less_optimal_peer, true, false);

        // Skip the peer optimality check and verify that the subscription is healthy
        assert!(subscription
            .check_subscription_health(&peers_and_metadata, true)
            .is_ok());

        // Verify that the last peer optimality check time has been
        // updated but that the last set of peers has not changed.
        let (_, last_optimality_check_peers) =
            subscription.last_optimality_check_time_and_peers.clone();
        verify_last_check_time_and_peers(
            &subscription,
            mock_time_service.now(),
            last_optimality_check_peers,
        );
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
    fn test_check_subscription_peer_optimality_refresh() {
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
        let time_now = mock_time_service.now();
        verify_subscription_syncing_progress(&mut subscription, first_synced_version, time_now);

        // Elapse some amount of time (not enough to timeout)
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_subscription_sync_timeout_ms / 2,
        ));

        // Verify that the DB is still making sync progress (we haven't timed out yet)
        verify_subscription_syncing_progress(&mut subscription, first_synced_version, time_now);

        // Elapse enough time to timeout the subscription
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_subscription_sync_timeout_ms + 1,
        ));

        // Verify that the DB is still making sync progress (the next version is higher)
        let time_now = mock_time_service.now();
        verify_subscription_syncing_progress(&mut subscription, second_synced_version, time_now);

        // Elapse some amount of time (not enough to timeout)
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_subscription_sync_timeout_ms / 2,
        ));

        // Verify that the DB is still making sync progress (we haven't timed out yet)
        verify_subscription_syncing_progress(&mut subscription, second_synced_version, time_now);

        // Elapse enough time to timeout the subscription
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_subscription_sync_timeout_ms + 1,
        ));

        // Verify that the DB is not making sync progress and that the subscription has timed out
        assert_matches!(
            subscription.check_syncing_progress(),
            Err(Error::SubscriptionProgressStopped(_))
        );
        assert_eq!(
            subscription.highest_synced_version_and_time,
            (second_synced_version, time_now)
        );
    }

    #[test]
    fn test_get_peer_network_id() {
        // Create a new observer subscription
        let consensus_observer_config = ConsensusObserverConfig::default();
        let peer_network_id = PeerNetworkId::random();
        let time_service = TimeService::mock();
        let subscription = ConsensusObserverSubscription::new(
            consensus_observer_config,
            Arc::new(MockDatabaseReader::new()),
            peer_network_id,
            time_service.clone(),
        );

        // Verify that the peer network id matches the expected value
        assert_eq!(subscription.get_peer_network_id(), peer_network_id);
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
        let result = subscription.check_subscription_peer_optimality(peers_and_metadata, false);

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
