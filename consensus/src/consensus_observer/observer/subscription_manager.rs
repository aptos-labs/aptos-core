// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    common::{
        error::Error,
        logging::{LogEntry, LogSchema},
        metrics,
    },
    network::{
        observer_client::ConsensusObserverClient,
        observer_message::{
            ConsensusObserverMessage, ConsensusObserverRequest, ConsensusObserverResponse,
        },
    },
    observer::{subscription, subscription::ConsensusObserverSubscription},
    publisher::consensus_publisher::ConsensusPublisher,
};
use aptos_config::{config::ConsensusObserverConfig, network_id::PeerNetworkId};
use aptos_logger::{error, info, warn};
use aptos_network::application::{interface::NetworkClient, metadata::PeerMetadata};
use aptos_storage_interface::DbReader;
use aptos_time_service::TimeService;
use std::{collections::HashMap, sync::Arc};

/// The manager for consensus observer subscriptions
pub struct SubscriptionManager {
    // The currently active consensus observer subscription
    active_observer_subscription: Option<ConsensusObserverSubscription>,

    // The consensus observer client to send network messages
    consensus_observer_client:
        Arc<ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>>,

    // The consensus observer configuration
    consensus_observer_config: ConsensusObserverConfig,

    // The consensus publisher
    consensus_publisher: Option<Arc<ConsensusPublisher>>,

    // A handle to storage (used to read the latest state and check progress)
    db_reader: Arc<dyn DbReader>,

    // The time service (used to check progress)
    time_service: TimeService,
}

impl SubscriptionManager {
    pub fn new(
        consensus_observer_client: Arc<
            ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
        >,
        consensus_observer_config: ConsensusObserverConfig,
        consensus_publisher: Option<Arc<ConsensusPublisher>>,
        db_reader: Arc<dyn DbReader>,
        time_service: TimeService,
    ) -> Self {
        Self {
            active_observer_subscription: None,
            consensus_observer_client,
            consensus_observer_config,
            consensus_publisher,
            db_reader,
            time_service,
        }
    }

    /// Checks if the active subscription is still healthy. If not, an error is returned.
    fn check_active_subscription(&mut self) -> Result<(), Error> {
        let active_observer_subscription = self.active_observer_subscription.take();
        if let Some(mut active_subscription) = active_observer_subscription {
            // Check if the peer for the subscription is still connected
            let peer_network_id = active_subscription.get_peer_network_id();
            let peer_still_connected = self
                .get_connected_peers_and_metadata()
                .map_or(false, |peers_and_metadata| {
                    peers_and_metadata.contains_key(&peer_network_id)
                });

            // Verify the peer is still connected
            if !peer_still_connected {
                return Err(Error::SubscriptionDisconnected(
                    "The peer is no longer connected!".to_string(),
                ));
            }

            // Verify the subscription has not timed out
            active_subscription.check_subscription_timeout()?;

            // Verify that the DB is continuing to sync and commit new data
            active_subscription.check_syncing_progress()?;

            // Verify that the subscription peer is optimal
            if let Some(peers_and_metadata) = self.get_connected_peers_and_metadata() {
                active_subscription.check_subscription_peer_optimality(peers_and_metadata)?;
            }

            // The subscription seems healthy, we can keep it
            self.active_observer_subscription = Some(active_subscription);
        }

        Ok(())
    }

    /// Checks the health of the active subscription. If the subscription is
    /// unhealthy, it will be terminated and a new subscription will be created.
    /// This returns true iff a new subscription was created.
    pub async fn check_and_manage_subscriptions(&mut self) -> bool {
        // Get the peer ID of the currently active subscription (if any)
        let active_subscription_peer = self
            .active_observer_subscription
            .as_ref()
            .map(|subscription| subscription.get_peer_network_id());

        // If we have an active subscription, verify that the subscription
        // is still healthy. If not, the subscription should be terminated.
        if let Some(active_subscription_peer) = active_subscription_peer {
            if let Err(error) = self.check_active_subscription() {
                // Log the subscription termination
                warn!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Terminating subscription to peer: {:?}! Error: {:?}",
                        active_subscription_peer, error
                    ))
                );

                // Unsubscribe from the peer
                self.unsubscribe_from_peer(active_subscription_peer);

                // Update the subscription termination metrics
                self.update_subscription_termination_metrics(active_subscription_peer, error);
            }
        }

        // If we don't have a subscription, we should select a new peer to
        // subscribe to. If we had a previous subscription (and it was
        // terminated) it should be excluded from the selection process.
        if self.active_observer_subscription.is_none() {
            // Create a new observer subscription
            self.create_new_observer_subscription(active_subscription_peer)
                .await;

            // If we successfully created a new subscription, update the metrics
            if let Some(active_subscription) = &self.active_observer_subscription {
                // Update the subscription creation metrics
                self.update_subscription_creation_metrics(
                    active_subscription.get_peer_network_id(),
                );

                return true; // A new subscription was created
            }
        }

        false // No new subscription was created
    }

    /// Creates a new observer subscription by sending subscription requests to
    /// appropriate peers and waiting for a successful response. If `previous_subscription_peer`
    /// is provided, it will be excluded from the selection process.
    async fn create_new_observer_subscription(
        &mut self,
        previous_subscription_peer: Option<PeerNetworkId>,
    ) {
        // Get a set of sorted peers to service our subscription request
        let sorted_peers = match self.sort_peers_for_subscription(previous_subscription_peer) {
            Some(sorted_peers) => sorted_peers,
            None => {
                error!(LogSchema::new(LogEntry::ConsensusObserver)
                    .message("Failed to sort peers for subscription requests!"));
                return;
            },
        };

        // Verify that we have potential peers
        if sorted_peers.is_empty() {
            warn!(LogSchema::new(LogEntry::ConsensusObserver)
                .message("There are no peers to subscribe to!"));
            return;
        }

        // Go through the sorted peers and attempt to subscribe to a single peer.
        // The first peer that responds successfully will be the selected peer.
        for selected_peer in &sorted_peers {
            info!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Attempting to subscribe to peer: {}!",
                    selected_peer
                ))
            );

            // Send a subscription request to the peer and wait for the response.
            // Note: it is fine to block here because we assume only a single active subscription.
            let subscription_request = ConsensusObserverRequest::Subscribe;
            let request_timeout_ms = self.consensus_observer_config.network_request_timeout_ms;
            let response = self
                .consensus_observer_client
                .send_rpc_request_to_peer(selected_peer, subscription_request, request_timeout_ms)
                .await;

            // Process the response and update the active subscription
            match response {
                Ok(ConsensusObserverResponse::SubscribeAck) => {
                    info!(
                        LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                            "Successfully subscribed to peer: {}!",
                            selected_peer
                        ))
                    );

                    // Update the active subscription
                    let subscription = ConsensusObserverSubscription::new(
                        self.consensus_observer_config,
                        self.db_reader.clone(),
                        *selected_peer,
                        self.time_service.clone(),
                    );
                    self.active_observer_subscription = Some(subscription);

                    return; // Return after successfully subscribing
                },
                Ok(response) => {
                    // We received an invalid response
                    warn!(
                        LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                            "Got unexpected response type: {:?}",
                            response.get_label()
                        ))
                    );
                },
                Err(error) => {
                    // We encountered an error while sending the request
                    error!(
                        LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                            "Failed to send subscription request to peer: {}! Error: {:?}",
                            selected_peer, error
                        ))
                    );
                },
            }
        }

        // We failed to connect to any peers
        warn!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Failed to subscribe to any peers! Num peers attempted: {:?}",
                sorted_peers.len()
            ))
        );
    }

    /// Gets the connected peers and metadata. If an error occurred,
    /// it is logged and None is returned.
    fn get_connected_peers_and_metadata(&self) -> Option<HashMap<PeerNetworkId, PeerMetadata>> {
        match self
            .consensus_observer_client
            .get_peers_and_metadata()
            .get_connected_peers_and_metadata()
        {
            Ok(connected_peers_and_metadata) => Some(connected_peers_and_metadata),
            Err(error) => {
                error!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Failed to get connected peers and metadata! Error: {:?}",
                        error
                    ))
                );
                None
            },
        }
    }

    /// Produces a list of sorted peers to service our subscription request.
    /// Note: if `previous_subscription_peer` is provided, it will be excluded
    /// from the selection process. Likewise, all peers currently subscribed to us
    /// will be excluded from the selection process.
    fn sort_peers_for_subscription(
        &mut self,
        previous_subscription_peer: Option<PeerNetworkId>,
    ) -> Option<Vec<PeerNetworkId>> {
        if let Some(mut peers_and_metadata) = self.get_connected_peers_and_metadata() {
            // Remove the previous subscription peer (if provided)
            if let Some(previous_subscription_peer) = previous_subscription_peer {
                let _ = peers_and_metadata.remove(&previous_subscription_peer);
            }

            // Remove any peers that are currently subscribed to us
            if let Some(consensus_publisher) = &self.consensus_publisher {
                for peer_network_id in consensus_publisher.get_active_subscribers() {
                    let _ = peers_and_metadata.remove(&peer_network_id);
                }
            }

            // Sort the peers by subscription optimality
            let sorted_peers =
                subscription::sort_peers_by_subscription_optimality(&peers_and_metadata);

            // Return the sorted peers
            Some(sorted_peers)
        } else {
            None // No connected peers were found
        }
    }

    /// Unsubscribes from the given peer by sending an unsubscribe request
    fn unsubscribe_from_peer(&self, peer_network_id: PeerNetworkId) {
        // Send an unsubscribe request to the peer and process the response.
        // Note: we execute this asynchronously, as we don't need to wait for the response.
        let consensus_observer_client = self.consensus_observer_client.clone();
        let consensus_observer_config = self.consensus_observer_config;
        tokio::spawn(async move {
            // Send the unsubscribe request to the peer
            let unsubscribe_request = ConsensusObserverRequest::Unsubscribe;
            let response = consensus_observer_client
                .send_rpc_request_to_peer(
                    &peer_network_id,
                    unsubscribe_request,
                    consensus_observer_config.network_request_timeout_ms,
                )
                .await;

            // Process the response
            match response {
                Ok(ConsensusObserverResponse::UnsubscribeAck) => {
                    info!(
                        LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                            "Successfully unsubscribed from peer: {}!",
                            peer_network_id
                        ))
                    );
                },
                Ok(response) => {
                    // We received an invalid response
                    warn!(
                        LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                            "Got unexpected response type: {:?}",
                            response.get_label()
                        ))
                    );
                },
                Err(error) => {
                    // We encountered an error while sending the request
                    error!(
                        LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                            "Failed to send unsubscribe request to peer: {}! Error: {:?}",
                            peer_network_id, error
                        ))
                    );
                },
            }
        });
    }

    /// Updates the subscription creation metrics for the given peer
    fn update_subscription_creation_metrics(&self, peer_network_id: PeerNetworkId) {
        // Set the number of active subscriptions
        metrics::set_gauge(
            &metrics::OBSERVER_NUM_ACTIVE_SUBSCRIPTIONS,
            &peer_network_id.network_id(),
            1,
        );

        // Update the number of created subscriptions
        metrics::increment_request_counter(
            &metrics::OBSERVER_CREATED_SUBSCRIPTIONS,
            metrics::CREATED_SUBSCRIPTION_LABEL,
            &peer_network_id,
        );
    }

    /// Updates the subscription termination metrics for the given peer
    fn update_subscription_termination_metrics(
        &self,
        peer_network_id: PeerNetworkId,
        error: Error,
    ) {
        // Reset the number of active subscriptions
        metrics::set_gauge(
            &metrics::OBSERVER_NUM_ACTIVE_SUBSCRIPTIONS,
            &peer_network_id.network_id(),
            0,
        );

        // Update the number of terminated subscriptions
        metrics::increment_request_counter(
            &metrics::OBSERVER_TERMINATED_SUBSCRIPTIONS,
            error.get_label(),
            &peer_network_id,
        );
    }

    /// Verifies that the message sender is the currently subscribed peer.
    /// If the sender is not the subscribed peer, an error is returned.
    pub fn verify_message_sender(&mut self, message_sender: PeerNetworkId) -> Result<(), Error> {
        if let Some(active_subscription) = &mut self.active_observer_subscription {
            active_subscription
                .verify_message_sender(&message_sender)
                .map_err(|error| {
                    // Send another unsubscription request to the peer (in case the previous was lost)
                    self.unsubscribe_from_peer(message_sender);
                    error
                })
        } else {
            // Send another unsubscription request to the peer (in case the previous was lost)
            self.unsubscribe_from_peer(message_sender);

            Err(Error::UnexpectedError(format!(
                "Received message from unexpected peer: {}! No active subscription found!",
                message_sender
            )))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_config::{config::PeerRole, network_id::NetworkId};
    use aptos_netcore::transport::ConnectionOrigin;
    use aptos_network::{
        application::storage::PeersAndMetadata,
        protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolId, ProtocolIdSet},
        transport::{ConnectionId, ConnectionMetadata},
    };
    use aptos_peer_monitoring_service_types::{
        response::NetworkInformationResponse, PeerMonitoringMetadata,
    };
    use aptos_types::{network_address::NetworkAddress, transaction::Version, PeerId};
    use claims::assert_matches;
    use maplit::hashmap;
    use mockall::mock;
    use std::{collections::BTreeMap, time::Duration};

    // This is a simple mock of the DbReader (it generates a MockDatabaseReader)
    mock! {
        pub DatabaseReader {}
        impl DbReader for DatabaseReader {
            fn get_latest_ledger_info_version(&self) -> aptos_storage_interface::Result<Version>;
        }
    }

    #[tokio::test]
    async fn test_check_active_subscription_connected() {
        // Create a consensus observer client
        let network_id = NetworkId::Public;
        let (peers_and_metadata, consensus_observer_client) =
            create_consensus_observer_client(&[network_id]);

        // Create a new subscription manager
        let consensus_observer_config = ConsensusObserverConfig::default();
        let db_reader = create_mock_db_reader();
        let mut subscription_manager = SubscriptionManager::new(
            consensus_observer_client,
            consensus_observer_config,
            None,
            db_reader.clone(),
            TimeService::mock(),
        );

        // Create a new subscription
        let observer_subscription = ConsensusObserverSubscription::new(
            consensus_observer_config,
            db_reader.clone(),
            PeerNetworkId::random(),
            TimeService::mock(),
        );
        subscription_manager.active_observer_subscription = Some(observer_subscription);

        // Check the active subscription and verify that it is removed (the peer is not connected)
        assert_matches!(
            subscription_manager.check_active_subscription(),
            Err(Error::SubscriptionDisconnected(_))
        );
        assert!(subscription_manager.active_observer_subscription.is_none());

        // Add a new connected peer
        let connected_peer =
            create_peer_and_connection(network_id, peers_and_metadata.clone(), 1, None, true);

        // Create a subscription to the new peer
        create_observer_subscription(
            &mut subscription_manager,
            consensus_observer_config,
            db_reader,
            connected_peer,
            TimeService::mock(),
        );

        // Check the active subscription and verify that it is still active (the peer is connected)
        assert!(subscription_manager.check_active_subscription().is_ok());
        let active_subscription = subscription_manager.active_observer_subscription.unwrap();
        assert_eq!(active_subscription.get_peer_network_id(), connected_peer);
    }

    #[tokio::test]
    async fn test_check_active_subscription_progress_stopped() {
        // Create a consensus observer config
        let consensus_observer_config = ConsensusObserverConfig {
            max_subscription_timeout_ms: 100_000_000, // Use a large value so that we don't time out
            ..ConsensusObserverConfig::default()
        };

        // Create a consensus observer client
        let network_id = NetworkId::Public;
        let (peers_and_metadata, consensus_observer_client) =
            create_consensus_observer_client(&[network_id]);

        // Create a new subscription manager
        let db_reader = create_mock_db_reader();
        let time_service = TimeService::mock();
        let mut subscription_manager = SubscriptionManager::new(
            consensus_observer_client,
            consensus_observer_config,
            None,
            db_reader.clone(),
            time_service.clone(),
        );

        // Add a new connected peer
        let connected_peer =
            create_peer_and_connection(network_id, peers_and_metadata.clone(), 1, None, true);

        // Create a subscription to the new peer
        create_observer_subscription(
            &mut subscription_manager,
            consensus_observer_config,
            db_reader.clone(),
            connected_peer,
            time_service.clone(),
        );

        // Elapse time to simulate a DB progress error
        let mock_time_service = time_service.clone().into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_synced_version_timeout_ms + 1,
        ));

        // Check the active subscription and verify that it is removed (the DB is not syncing)
        assert_matches!(
            subscription_manager.check_active_subscription(),
            Err(Error::SubscriptionProgressStopped(_))
        );
        assert!(subscription_manager.active_observer_subscription.is_none());
    }

    #[tokio::test]
    async fn test_check_active_subscription_timeout() {
        // Create a consensus observer client
        let network_id = NetworkId::Public;
        let (peers_and_metadata, consensus_observer_client) =
            create_consensus_observer_client(&[network_id]);

        // Create a new subscription manager
        let consensus_observer_config = ConsensusObserverConfig::default();
        let db_reader = create_mock_db_reader();
        let time_service = TimeService::mock();
        let mut subscription_manager = SubscriptionManager::new(
            consensus_observer_client,
            consensus_observer_config,
            None,
            db_reader.clone(),
            time_service.clone(),
        );

        // Add a new connected peer
        let connected_peer =
            create_peer_and_connection(network_id, peers_and_metadata.clone(), 1, None, true);

        // Create a subscription to the new peer
        create_observer_subscription(
            &mut subscription_manager,
            consensus_observer_config,
            db_reader.clone(),
            connected_peer,
            time_service.clone(),
        );

        // Elapse time to simulate a timeout
        let mock_time_service = time_service.clone().into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_subscription_timeout_ms + 1,
        ));

        // Check the active subscription and verify that it is removed (the subscription timed out)
        assert_matches!(
            subscription_manager.check_active_subscription(),
            Err(Error::SubscriptionTimeout(_))
        );
        assert!(subscription_manager.active_observer_subscription.is_none());
    }

    #[tokio::test]
    async fn test_check_active_subscription_suboptimal() {
        // Create a consensus observer config
        let consensus_observer_config = ConsensusObserverConfig {
            max_subscription_timeout_ms: 100_000_000, // Use a large value so that we don't time out
            max_synced_version_timeout_ms: 100_000_000, // Use a large value so that we don't get DB progress errors
            ..ConsensusObserverConfig::default()
        };

        // Create a consensus observer client
        let network_id = NetworkId::Validator;
        let (peers_and_metadata, consensus_observer_client) =
            create_consensus_observer_client(&[network_id]);

        // Create a new subscription manager
        let db_reader = create_mock_db_reader();
        let time_service = TimeService::mock();
        let mut subscription_manager = SubscriptionManager::new(
            consensus_observer_client,
            consensus_observer_config,
            None,
            db_reader.clone(),
            time_service.clone(),
        );

        // Add an optimal validator peer
        create_peer_and_connection(network_id, peers_and_metadata.clone(), 0, Some(0.1), true);

        // Add a suboptimal validator peer
        let suboptimal_peer =
            create_peer_and_connection(network_id, peers_and_metadata.clone(), 0, None, true);

        // Create a new subscription to the suboptimal peer
        create_observer_subscription(
            &mut subscription_manager,
            consensus_observer_config,
            db_reader.clone(),
            suboptimal_peer,
            time_service.clone(),
        );

        // Elapse enough time to trigger the peer optimality check
        let mock_time_service = time_service.clone().into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_peer_change_interval_ms + 1,
        ));

        // Check the active subscription and verify that it is removed (the peer is suboptimal)
        assert_matches!(
            subscription_manager.check_active_subscription(),
            Err(Error::SubscriptionSuboptimal(_))
        );
        assert!(subscription_manager.active_observer_subscription.is_none());
    }

    #[tokio::test]
    async fn test_sort_peers_for_subscription() {
        // Create a consensus observer client
        let network_ids = &[NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
        let (peers_and_metadata, consensus_observer_client) =
            create_consensus_observer_client(network_ids);

        // Create a new subscription manager
        let consensus_observer_config = ConsensusObserverConfig::default();
        let db_reader = create_mock_db_reader();
        let mut subscription_manager = SubscriptionManager::new(
            consensus_observer_client,
            consensus_observer_config,
            None,
            db_reader.clone(),
            TimeService::mock(),
        );

        // Sort the peers for a subscription and verify that no peers are returned
        let sorted_peers = subscription_manager
            .sort_peers_for_subscription(None)
            .unwrap();
        assert!(sorted_peers.is_empty());

        // Add a connected validator peer, VFN peer and public peer
        for network_id in network_ids {
            let distance_from_validators = match network_id {
                NetworkId::Validator => 0,
                NetworkId::Vfn => 1,
                NetworkId::Public => 2,
            };
            create_peer_and_connection(
                *network_id,
                peers_and_metadata.clone(),
                distance_from_validators,
                None,
                true,
            );
        }

        // Sort the peers for a subscription and verify the ordering (according to distance)
        let sorted_peers = subscription_manager
            .sort_peers_for_subscription(None)
            .unwrap();
        assert_eq!(sorted_peers[0].network_id(), NetworkId::Validator);
        assert_eq!(sorted_peers[1].network_id(), NetworkId::Vfn);
        assert_eq!(sorted_peers[2].network_id(), NetworkId::Public);
        assert_eq!(sorted_peers.len(), 3);

        // Sort the peers, but mark the validator as the last subscribed peer
        let previous_subscription_peer = sorted_peers[0];
        let sorted_peer_subset = subscription_manager
            .sort_peers_for_subscription(Some(previous_subscription_peer))
            .unwrap();
        assert_eq!(sorted_peer_subset[0].network_id(), NetworkId::Vfn);
        assert_eq!(sorted_peer_subset[1].network_id(), NetworkId::Public);
        assert_eq!(sorted_peer_subset.len(), 2);

        // Remove all the peers and verify that no peers are returned
        for peer_network_id in sorted_peers {
            remove_peer_and_connection(peers_and_metadata.clone(), peer_network_id);
        }

        // Add multiple validator peers, with different latencies
        let mut validator_peers = vec![];
        for ping_latency_secs in [0.9, 0.8, 0.5, 0.1, 0.05] {
            let validator_peer = create_peer_and_connection(
                NetworkId::Validator,
                peers_and_metadata.clone(),
                0,
                Some(ping_latency_secs),
                true,
            );
            validator_peers.push(validator_peer);
        }

        // Sort the peers for a subscription and verify the ordering (according to latency)
        let sorted_peers = subscription_manager
            .sort_peers_for_subscription(None)
            .unwrap();
        let expected_peers = validator_peers.into_iter().rev().collect::<Vec<_>>();
        assert_eq!(sorted_peers, expected_peers);
    }

    #[tokio::test]
    async fn test_verify_message_sender() {
        // Create a consensus observer client
        let network_id = NetworkId::Public;
        let (_, consensus_observer_client) = create_consensus_observer_client(&[network_id]);

        // Create a new subscription manager
        let consensus_observer_config = ConsensusObserverConfig::default();
        let db_reader = Arc::new(MockDatabaseReader::new());
        let mut subscription_manager = SubscriptionManager::new(
            consensus_observer_client,
            consensus_observer_config,
            None,
            db_reader.clone(),
            TimeService::mock(),
        );

        // Check that message verification fails (we have no active subscription)
        assert!(subscription_manager
            .verify_message_sender(PeerNetworkId::random())
            .is_err());

        // Create a new subscription
        let subscription_peer = PeerNetworkId::random();
        create_observer_subscription(
            &mut subscription_manager,
            consensus_observer_config,
            db_reader.clone(),
            subscription_peer,
            TimeService::mock(),
        );

        // Check that message verification fails if the peer doesn't match the subscription
        assert!(subscription_manager
            .verify_message_sender(PeerNetworkId::random())
            .is_err());

        // Check that message verification passes if the peer matches the subscription
        assert!(subscription_manager
            .verify_message_sender(subscription_peer)
            .is_ok());
    }

    /// Creates a new consensus observer client and a peers and metadata container
    fn create_consensus_observer_client(
        network_ids: &[NetworkId],
    ) -> (
        Arc<PeersAndMetadata>,
        Arc<ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>>,
    ) {
        let peers_and_metadata = PeersAndMetadata::new(network_ids);
        let network_client =
            NetworkClient::new(vec![], vec![], hashmap![], peers_and_metadata.clone());
        let consensus_observer_client = Arc::new(ConsensusObserverClient::new(network_client));

        (peers_and_metadata, consensus_observer_client)
    }

    /// Creates a mock DB reader that always returns 0 for the latest version
    fn create_mock_db_reader() -> Arc<MockDatabaseReader> {
        let mut mock_db_reader = MockDatabaseReader::new();
        mock_db_reader
            .expect_get_latest_ledger_info_version()
            .returning(move || Ok(0));
        Arc::new(mock_db_reader)
    }

    /// Creates a new observer subscription for the specified peer
    fn create_observer_subscription(
        subscription_manager: &mut SubscriptionManager,
        consensus_observer_config: ConsensusObserverConfig,
        db_reader: Arc<MockDatabaseReader>,
        subscription_peer: PeerNetworkId,
        time_service: TimeService,
    ) {
        let observer_subscription = ConsensusObserverSubscription::new(
            consensus_observer_config,
            db_reader.clone(),
            subscription_peer,
            time_service,
        );
        subscription_manager.active_observer_subscription = Some(observer_subscription);
    }

    /// Creates a new peer with the specified connection metadata
    fn create_peer_and_connection(
        network_id: NetworkId,
        peers_and_metadata: Arc<PeersAndMetadata>,
        distance_from_validators: u64,
        ping_latency_secs: Option<f64>,
        support_consensus_observer: bool,
    ) -> PeerNetworkId {
        // Create the connection metadata
        let peer_network_id = PeerNetworkId::new(network_id, PeerId::random());
        let connection_metadata = if support_consensus_observer {
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
        };

        // Insert the connection into peers and metadata
        peers_and_metadata
            .insert_connection_metadata(peer_network_id, connection_metadata.clone())
            .unwrap();

        // Update the peer monitoring metadata
        let latest_network_info_response = NetworkInformationResponse {
            connected_peers: BTreeMap::new(),
            distance_from_validators,
        };
        let monitoring_metdata = PeerMonitoringMetadata::new(
            ping_latency_secs,
            ping_latency_secs,
            Some(latest_network_info_response),
            None,
            None,
        );
        peers_and_metadata
            .update_peer_monitoring_metadata(peer_network_id, monitoring_metdata.clone())
            .unwrap();

        peer_network_id
    }

    /// Removes the peer and connection metadata for the given peer
    fn remove_peer_and_connection(
        peers_and_metadata: Arc<PeersAndMetadata>,
        peer_network_id: PeerNetworkId,
    ) {
        let peer_metadata = peers_and_metadata
            .get_metadata_for_peer(peer_network_id)
            .unwrap();
        let connection_id = peer_metadata.get_connection_metadata().connection_id;
        peers_and_metadata
            .remove_peer_metadata(peer_network_id, connection_id)
            .unwrap();
    }
}
