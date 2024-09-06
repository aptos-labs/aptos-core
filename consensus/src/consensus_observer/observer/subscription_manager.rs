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
use itertools::Itertools;
use std::{collections::HashMap, sync::Arc};

/// The manager for consensus observer subscriptions
pub struct SubscriptionManager {
    // The currently active set of consensus observer subscriptions
    active_observer_subscriptions: HashMap<PeerNetworkId, ConsensusObserverSubscription>,

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
            active_observer_subscriptions: HashMap::new(),
            consensus_observer_client,
            consensus_observer_config,
            consensus_publisher,
            db_reader,
            time_service,
        }
    }

    /// Checks if the subscription to the given peer is still healthy.
    /// If not, an error explaining why it is unhealthy is returned.
    fn check_subscription_health(
        &mut self,
        connected_peers_and_metadata: &HashMap<PeerNetworkId, PeerMetadata>,
        peer_network_id: PeerNetworkId,
    ) -> Result<(), Error> {
        match self.active_observer_subscriptions.get_mut(&peer_network_id) {
            Some(active_subscription) => {
                // Verify the peer is still connected
                if !connected_peers_and_metadata.contains_key(&peer_network_id) {
                    return Err(Error::SubscriptionDisconnected(format!(
                        "The peer: {:?} is no longer connected!",
                        peer_network_id
                    )));
                }

                // Verify the subscription has not timed out
                active_subscription.check_subscription_timeout()?;

                // Verify that the DB is continuing to sync and commit new data
                active_subscription.check_syncing_progress()?;

                // Verify that the subscription peer is still optimal
                active_subscription
                    .check_subscription_peer_optimality(connected_peers_and_metadata)?;

                // The subscription seems healthy
                Ok(())
            },
            None => Err(Error::UnexpectedError(format!(
                "The subscription to peer: {:?} is not active!",
                peer_network_id
            ))),
        }
    }

    /// Checks the health of the active subscriptions. If any subscription is
    /// unhealthy, it will be terminated and new subscriptions will be created.
    /// This returns an error iff all subscriptions were unhealthy and terminated.
    pub async fn check_and_manage_subscriptions(&mut self) -> Result<(), Error> {
        // Get the subscription and connected peers
        let initial_subscription_peers = self.get_active_subscription_peers();
        let connected_peers_and_metadata = self.get_connected_peers_and_metadata();

        // Terminate any unhealthy subscriptions
        let terminated_subscriptions =
            self.terminate_unhealthy_subscriptions(&connected_peers_and_metadata);

        // Check if all subscriptions were terminated
        let num_terminated_subscriptions = terminated_subscriptions.len();
        let all_subscriptions_terminated = num_terminated_subscriptions > 0
            && num_terminated_subscriptions == initial_subscription_peers.len();

        // Calculate the number of new subscriptions to create
        let max_concurrent_subscriptions =
            self.consensus_observer_config.max_concurrent_subscriptions as usize;
        let num_subscriptions_to_create =
            max_concurrent_subscriptions.saturating_sub(self.active_observer_subscriptions.len());

        // Create the new subscriptions (if required)
        let terminated_subscription_peers = terminated_subscriptions
            .iter()
            .map(|(peer, _)| *peer)
            .collect();
        let new_subscription_peers = self
            .create_new_subscriptions(
                connected_peers_and_metadata,
                num_subscriptions_to_create,
                terminated_subscription_peers,
            )
            .await;

        // Log a warning if we failed to create as many subscriptions as requested
        let num_subscriptions_created = new_subscription_peers.len();
        if num_subscriptions_created < num_subscriptions_to_create {
            warn!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Failed to create the requested number of subscriptions! Number of subscriptions \
                    requested: {:?}, number of subscriptions created: {:?}.",
                    num_subscriptions_to_create,
                    num_subscriptions_created
                ))
            );
        }

        // Update the subscription metrics
        self.update_subscription_metrics(&new_subscription_peers, terminated_subscriptions);

        // Return an error if all subscriptions were terminated
        if all_subscriptions_terminated {
            Err(Error::SubscriptionsReset(format!(
                "All subscriptions were unhealthy and terminated! Number of terminated \
                    subscriptions: {:?}, number of new subscriptions created: {:?}.",
                num_terminated_subscriptions, num_subscriptions_created,
            )))
        } else {
            Ok(())
        }
    }

    /// Attempts to create the given number of new subscriptions
    /// and returns the peer IDs of the newly created subscriptions.
    /// Any `unhealthy_subscription_peers` are excluded from selection.
    async fn create_new_subscriptions(
        &mut self,
        connected_peers_and_metadata: HashMap<PeerNetworkId, PeerMetadata>,
        num_subscriptions_to_create: usize,
        unhealthy_subscription_peers: Vec<PeerNetworkId>,
    ) -> Vec<PeerNetworkId> {
        // Return early if we don't need to create any new subscriptions
        if num_subscriptions_to_create == 0 {
            return vec![];
        }

        // Sort the potential peers for subscription requests
        let mut sorted_potential_peers = match self
            .sort_peers_for_subscription(connected_peers_and_metadata, unhealthy_subscription_peers)
        {
            Some(sorted_peers) => sorted_peers,
            None => {
                error!(LogSchema::new(LogEntry::ConsensusObserver)
                    .message("Failed to sort peers for subscription requests!"));
                return vec![];
            },
        };

        // Verify that we have potential peers to subscribe to
        if sorted_potential_peers.is_empty() {
            warn!(LogSchema::new(LogEntry::ConsensusObserver)
                .message("There are no potential peers to subscribe to!"));
            return vec![];
        }

        // Go through the potential peers and attempt to create new subscriptions
        let mut created_subscription_peers = vec![];
        for _ in 0..num_subscriptions_to_create {
            if let Some(subscription_peer) = self
                .create_single_subscription(sorted_potential_peers.clone())
                .await
            {
                // Add the peer to the list of created subscriptions
                created_subscription_peers.push(subscription_peer);

                // Remove the peer from the sorted list (for the next selection)
                sorted_potential_peers.retain(|peer| peer != &subscription_peer);
            }
        }

        // Return the list of created subscriptions
        created_subscription_peers
    }

    /// Attempts to create a new subscription to a single peer from
    /// the sorted list of potential peers. If a new subscription is
    /// successfully created, the peer is returned.
    async fn create_single_subscription(
        &mut self,
        sorted_potential_peers: Vec<PeerNetworkId>,
    ) -> Option<PeerNetworkId> {
        for potential_peer in sorted_potential_peers {
            // Log the subscription attempt
            info!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Attempting to subscribe to potential peer: {}!",
                    potential_peer
                ))
            );

            // Send a subscription request to the peer and wait for the response.
            // TODO: we should make this non-blocking!
            let subscription_request = ConsensusObserverRequest::Subscribe;
            let request_timeout_ms = self.consensus_observer_config.network_request_timeout_ms;
            let response = self
                .consensus_observer_client
                .send_rpc_request_to_peer(&potential_peer, subscription_request, request_timeout_ms)
                .await;

            // Process the response and update the active subscription
            match response {
                Ok(ConsensusObserverResponse::SubscribeAck) => {
                    // Log the successful subscription
                    info!(
                        LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                            "Successfully subscribed to peer: {}!",
                            potential_peer
                        ))
                    );

                    // Create the new subscription
                    let subscription = ConsensusObserverSubscription::new(
                        self.consensus_observer_config,
                        self.db_reader.clone(),
                        potential_peer,
                        self.time_service.clone(),
                    );

                    // Add the subscription to the active subscriptions
                    self.active_observer_subscriptions
                        .insert(potential_peer, subscription);

                    // Return the successful subscription peer
                    return Some(potential_peer);
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
                            potential_peer, error
                        ))
                    );
                },
            }
        }

        // We failed to create a new subscription
        None
    }

    /// Returns the currently active subscription peers
    fn get_active_subscription_peers(&self) -> Vec<PeerNetworkId> {
        self.active_observer_subscriptions.keys().cloned().collect()
    }

    /// Gets the connected peers and metadata. If an error
    /// occurred, it is logged and an empty map is returned.
    fn get_connected_peers_and_metadata(&self) -> HashMap<PeerNetworkId, PeerMetadata> {
        self.consensus_observer_client
            .get_peers_and_metadata()
            .get_connected_peers_and_metadata()
            .unwrap_or_else(|error| {
                // Log the error
                error!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Failed to get connected peers and metadata! Error: {:?}",
                        error
                    ))
                );

                // Return an empty map
                HashMap::new()
            })
    }

    /// Produces a list of sorted peers to service our subscription requests.
    /// Note: if `unhealthy_subscription_peers` are provided, they will be excluded
    /// from the selection process. Likewise, all peers currently subscribed to us
    /// will be excluded from the selection process.
    fn sort_peers_for_subscription(
        &mut self,
        mut connected_peers_and_metadata: HashMap<PeerNetworkId, PeerMetadata>,
        unhealthy_subscription_peers: Vec<PeerNetworkId>,
    ) -> Option<Vec<PeerNetworkId>> {
        // Remove any peers we're already subscribed to
        for active_subscription_peer in self.get_active_subscription_peers() {
            let _ = connected_peers_and_metadata.remove(&active_subscription_peer);
        }

        // Remove any unhealthy subscription peers
        for unhealthy_peer in unhealthy_subscription_peers {
            let _ = connected_peers_and_metadata.remove(&unhealthy_peer);
        }

        // Remove any peers that are currently subscribed to us
        if let Some(consensus_publisher) = &self.consensus_publisher {
            for peer_network_id in consensus_publisher.get_active_subscribers() {
                let _ = connected_peers_and_metadata.remove(&peer_network_id);
            }
        }

        // Sort the peers by subscription optimality
        let sorted_peers =
            subscription::sort_peers_by_subscription_optimality(&connected_peers_and_metadata);

        // Return the sorted peers
        Some(sorted_peers)
    }

    /// Terminates any unhealthy subscriptions and returns the list of terminated subscriptions
    fn terminate_unhealthy_subscriptions(
        &mut self,
        connected_peers_and_metadata: &HashMap<PeerNetworkId, PeerMetadata>,
    ) -> Vec<(PeerNetworkId, Error)> {
        let mut terminated_subscriptions = vec![];
        for subscription_peer in self.get_active_subscription_peers() {
            // Check the health of the subscription and terminate it if needed
            if let Err(error) =
                self.check_subscription_health(connected_peers_and_metadata, subscription_peer)
            {
                // Log the subscription termination error
                warn!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Terminating subscription to peer: {:?}! Termination reason: {:?}",
                        subscription_peer, error
                    ))
                );

                // Unsubscribe from the peer and remove the subscription
                self.unsubscribe_from_peer(subscription_peer);

                // Add the peer to the list of terminated subscriptions
                terminated_subscriptions.push((subscription_peer, error));
            }
        }

        terminated_subscriptions
    }

    /// Unsubscribes from the given peer by sending an unsubscribe request
    fn unsubscribe_from_peer(&mut self, peer_network_id: PeerNetworkId) {
        // Remove the peer from the active subscriptions
        self.active_observer_subscriptions.remove(&peer_network_id);

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

    /// Updates the subscription creation and termination metrics
    fn update_subscription_metrics(
        &self,
        new_subscription_peers: &[PeerNetworkId],
        terminated_subscription_peers: Vec<(PeerNetworkId, Error)>,
    ) {
        // Update the created subscriptions metrics
        for peer_network_id in new_subscription_peers {
            metrics::increment_counter(
                &metrics::OBSERVER_CREATED_SUBSCRIPTIONS,
                metrics::CREATED_SUBSCRIPTION_LABEL,
                peer_network_id,
            );
        }

        // Update the terminated subscriptions metrics
        for (peer_network_id, termination_reason) in terminated_subscription_peers {
            metrics::increment_counter(
                &metrics::OBSERVER_TERMINATED_SUBSCRIPTIONS,
                termination_reason.get_label(),
                &peer_network_id,
            );
        }

        // Set the number of active subscriptions (grouped by network ID)
        let active_subscription_peers = self.get_active_subscription_peers();
        for (network_id, active_subscription_peers) in &active_subscription_peers
            .iter()
            .chunk_by(|peer_network_id| peer_network_id.network_id())
        {
            metrics::set_gauge(
                &metrics::OBSERVER_NUM_ACTIVE_SUBSCRIPTIONS,
                &network_id,
                active_subscription_peers.collect::<Vec<_>>().len() as i64,
            );
        }
    }

    /// Verifies that the message is from an active subscription.
    /// If not, an error is returned.
    pub fn verify_message_for_subscription(
        &mut self,
        message_sender: PeerNetworkId,
    ) -> Result<(), Error> {
        match self.active_observer_subscriptions.get_mut(&message_sender) {
            Some(active_subscription) => {
                // The message is from an active subscription (update the last message time)
                active_subscription.update_last_message_receive_time();
                Ok(())
            },
            None => {
                // The message is not from an active subscription (send another unsubscribe request)
                self.unsubscribe_from_peer(message_sender);
                Err(Error::UnexpectedError(format!(
                    "Received message from unexpected peer, and not an active subscription: {}!",
                    message_sender
                )))
            },
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
        let peer_network_id = PeerNetworkId::random();
        let observer_subscription = ConsensusObserverSubscription::new(
            consensus_observer_config,
            db_reader.clone(),
            peer_network_id,
            TimeService::mock(),
        );
        subscription_manager.active_observer_subscriptions =
            hashmap! {peer_network_id => observer_subscription};

        // Check the active subscription and verify that it is removed (the peer is not connected)
        let connected_peers_and_metadata = peers_and_metadata
            .get_connected_peers_and_metadata()
            .unwrap();
        assert_matches!(
            subscription_manager
                .check_subscription_health(&connected_peers_and_metadata, peer_network_id),
            Err(Error::SubscriptionDisconnected(_))
        );
        assert!(subscription_manager
            .active_observer_subscriptions
            .is_empty());

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

        // Check the active subscription is still healthy
        let connected_peers_and_metadata = peers_and_metadata
            .get_connected_peers_and_metadata()
            .unwrap();
        assert!(subscription_manager
            .check_subscription_health(&connected_peers_and_metadata, connected_peer)
            .is_ok());

        // Verify that the active subscription is still present
        assert!(subscription_manager
            .active_observer_subscriptions
            .contains_key(&connected_peer));
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
        let connected_peers_and_metadata = peers_and_metadata
            .get_connected_peers_and_metadata()
            .unwrap();
        assert_matches!(
            subscription_manager
                .check_subscription_health(&connected_peers_and_metadata, connected_peer),
            Err(Error::SubscriptionProgressStopped(_))
        );
        assert!(subscription_manager
            .active_observer_subscriptions
            .is_empty());
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
        let connected_peers_and_metadata = peers_and_metadata
            .get_connected_peers_and_metadata()
            .unwrap();
        assert_matches!(
            subscription_manager
                .check_subscription_health(&connected_peers_and_metadata, connected_peer),
            Err(Error::SubscriptionTimeout(_))
        );
        assert!(subscription_manager
            .active_observer_subscriptions
            .is_empty());
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
        let connected_peers_and_metadata = peers_and_metadata
            .get_connected_peers_and_metadata()
            .unwrap();
        assert_matches!(
            subscription_manager
                .check_subscription_health(&connected_peers_and_metadata, suboptimal_peer),
            Err(Error::SubscriptionSuboptimal(_))
        );
        assert!(subscription_manager
            .active_observer_subscriptions
            .is_empty());
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
        let connected_peers_and_metadata = peers_and_metadata
            .get_connected_peers_and_metadata()
            .unwrap();
        let sorted_peers = subscription_manager
            .sort_peers_for_subscription(connected_peers_and_metadata, vec![])
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
        let connected_peers_and_metadata = peers_and_metadata
            .get_connected_peers_and_metadata()
            .unwrap();
        let sorted_peers = subscription_manager
            .sort_peers_for_subscription(connected_peers_and_metadata, vec![])
            .unwrap();
        assert_eq!(sorted_peers[0].network_id(), NetworkId::Validator);
        assert_eq!(sorted_peers[1].network_id(), NetworkId::Vfn);
        assert_eq!(sorted_peers[2].network_id(), NetworkId::Public);
        assert_eq!(sorted_peers.len(), 3);

        // Sort the peers, but mark the validator as the last subscribed peer
        let connected_peers_and_metadata = peers_and_metadata
            .get_connected_peers_and_metadata()
            .unwrap();
        let sorted_peer_subset = subscription_manager
            .sort_peers_for_subscription(connected_peers_and_metadata, vec![sorted_peers[0]])
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
        let connected_peers_and_metadata = peers_and_metadata
            .get_connected_peers_and_metadata()
            .unwrap();
        let sorted_peers = subscription_manager
            .sort_peers_for_subscription(connected_peers_and_metadata, vec![])
            .unwrap();
        let expected_peers = validator_peers.into_iter().rev().collect::<Vec<_>>();
        assert_eq!(sorted_peers, expected_peers);
    }

    #[tokio::test]
    async fn test_verify_message_from_subscription() {
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
            .verify_message_for_subscription(PeerNetworkId::random())
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
            .verify_message_for_subscription(PeerNetworkId::random())
            .is_err());

        // Check that message verification passes if the peer matches the subscription
        assert!(subscription_manager
            .verify_message_for_subscription(subscription_peer)
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
        subscription_manager.active_observer_subscriptions =
            hashmap! {subscription_peer => observer_subscription};
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
