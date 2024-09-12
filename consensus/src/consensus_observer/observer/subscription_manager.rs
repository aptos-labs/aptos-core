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
    observer::{subscription::ConsensusObserverSubscription, subscription_utils},
    publisher::consensus_publisher::ConsensusPublisher,
};
use aptos_config::{config::ConsensusObserverConfig, network_id::PeerNetworkId};
use aptos_infallible::Mutex;
use aptos_logger::{error, info, warn};
use aptos_network::application::{interface::NetworkClient, metadata::PeerMetadata};
use aptos_storage_interface::DbReader;
use aptos_time_service::TimeService;
use itertools::Itertools;
use std::{collections::HashMap, sync::Arc};
use tokio::task::JoinHandle;

/// The manager for consensus observer subscriptions
pub struct SubscriptionManager {
    // The currently active set of consensus observer subscriptions
    active_observer_subscriptions:
        Arc<Mutex<HashMap<PeerNetworkId, ConsensusObserverSubscription>>>,

    // The active subscription creation task (if one is currently running)
    active_subscription_creation_task: Arc<Mutex<Option<JoinHandle<()>>>>,

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
            active_observer_subscriptions: Arc::new(Mutex::new(HashMap::new())),
            active_subscription_creation_task: Arc::new(Mutex::new(None)),
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
        // Get the active subscription for the peer
        let mut active_observer_subscriptions = self.active_observer_subscriptions.lock();
        let active_subscription = active_observer_subscriptions.get_mut(&peer_network_id);

        // Check the health of the subscription
        match active_subscription {
            Some(active_subscription) => {
                active_subscription.check_subscription_health(connected_peers_and_metadata)
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
        let remaining_subscription_peers = self.get_active_subscription_peers();
        let max_concurrent_subscriptions =
            self.consensus_observer_config.max_concurrent_subscriptions as usize;
        let num_subscriptions_to_create =
            max_concurrent_subscriptions.saturating_sub(remaining_subscription_peers.len());

        // Update the total subscription metrics
        update_total_subscription_metrics(&remaining_subscription_peers);

        // Spawn a task to create the new subscriptions (asynchronously)
        self.spawn_subscription_creation_task(
            num_subscriptions_to_create,
            remaining_subscription_peers,
            terminated_subscriptions,
            connected_peers_and_metadata,
        )
        .await;

        // Return an error if all subscriptions were terminated
        if all_subscriptions_terminated {
            Err(Error::SubscriptionsReset(format!(
                "All {:?} subscriptions were unhealthy and terminated!",
                num_terminated_subscriptions,
            )))
        } else {
            Ok(())
        }
    }

    /// Returns the currently active subscription peers
    fn get_active_subscription_peers(&self) -> Vec<PeerNetworkId> {
        let active_observer_subscriptions = self.active_observer_subscriptions.lock();
        active_observer_subscriptions.keys().cloned().collect()
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

    /// Spawns a new subscription creation task to create
    /// the specified number of new subscriptions.
    async fn spawn_subscription_creation_task(
        &mut self,
        num_subscriptions_to_create: usize,
        active_subscription_peers: Vec<PeerNetworkId>,
        terminated_subscriptions: Vec<(PeerNetworkId, Error)>,
        connected_peers_and_metadata: HashMap<PeerNetworkId, PeerMetadata>,
    ) {
        // If there are no new subscriptions to create, return early
        if num_subscriptions_to_create == 0 {
            return;
        }

        // If there is an active subscription creation task, return early
        if let Some(subscription_creation_task) = &*self.active_subscription_creation_task.lock() {
            if !subscription_creation_task.is_finished() {
                return; // The task is still running
            }
        }

        // Clone the shared state for the task
        let active_observer_subscriptions = self.active_observer_subscriptions.clone();
        let consensus_observer_config = self.consensus_observer_config;
        let consensus_observer_client = self.consensus_observer_client.clone();
        let consensus_publisher = self.consensus_publisher.clone();
        let db_reader = self.db_reader.clone();
        let time_service = self.time_service.clone();

        // Spawn a new subscription creation task
        let subscription_creation_task = tokio::spawn(async move {
            // Identify the terminated subscription peers
            let terminated_subscription_peers = terminated_subscriptions
                .iter()
                .map(|(peer, _)| *peer)
                .collect();

            // Create the new subscriptions
            let new_subscriptions = subscription_utils::create_new_subscriptions(
                consensus_observer_config,
                consensus_observer_client,
                consensus_publisher,
                db_reader,
                time_service,
                connected_peers_and_metadata,
                num_subscriptions_to_create,
                active_subscription_peers,
                terminated_subscription_peers,
            )
            .await;

            // Identify the new subscription peers
            let new_subscription_peers = new_subscriptions
                .iter()
                .map(|subscription| subscription.get_peer_network_id())
                .collect::<Vec<_>>();

            // Add the new subscriptions to the list of active subscriptions
            for subscription in new_subscriptions {
                active_observer_subscriptions
                    .lock()
                    .insert(subscription.get_peer_network_id(), subscription);
            }

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

            // Update the subscription change metrics
            update_subscription_change_metrics(new_subscription_peers, terminated_subscriptions);
        });

        // Update the active subscription creation task
        *self.active_subscription_creation_task.lock() = Some(subscription_creation_task);
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
        self.active_observer_subscriptions
            .lock()
            .remove(&peer_network_id);

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

    /// Verifies that the message is from an active
    /// subscription. If not, an error is returned.
    pub fn verify_message_for_subscription(
        &mut self,
        message_sender: PeerNetworkId,
    ) -> Result<(), Error> {
        // Check if the message is from an active subscription
        if let Some(active_subscription) = self
            .active_observer_subscriptions
            .lock()
            .get_mut(&message_sender)
        {
            // Update the last message receive time and return early
            active_subscription.update_last_message_receive_time();
            return Ok(());
        }

        // Otherwise, the message is not from an active subscription.
        // Send another unsubscribe request, and return an error.
        self.unsubscribe_from_peer(message_sender);
        Err(Error::InvalidMessageError(format!(
            "Received message from unexpected peer, and not an active subscription: {}!",
            message_sender
        )))
    }
}

/// Updates the subscription creation and termination metrics
fn update_subscription_change_metrics(
    new_subscription_peers: Vec<PeerNetworkId>,
    terminated_subscription_peers: Vec<(PeerNetworkId, Error)>,
) {
    // Update the created subscriptions metrics
    for peer_network_id in new_subscription_peers {
        metrics::increment_counter(
            &metrics::OBSERVER_CREATED_SUBSCRIPTIONS,
            metrics::CREATED_SUBSCRIPTION_LABEL,
            &peer_network_id,
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
}

/// Updates the total subscription metrics (grouped by network ID)
fn update_total_subscription_metrics(active_subscription_peers: &[PeerNetworkId]) {
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
    async fn test_check_subscription_health_connected() {
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
        create_observer_subscription(
            &mut subscription_manager,
            consensus_observer_config,
            db_reader.clone(),
            peer_network_id,
            TimeService::mock(),
        );

        // Check the active subscription and verify that it unhealthy (the peer is not connected)
        check_subscription_connection(&mut subscription_manager, peer_network_id, false);

        // Terminate the subscription
        let terminated_subscriptions =
            terminate_any_unhealthy_subscriptions(&mut subscription_manager);
        assert_eq!(terminated_subscriptions.len(), 1);
        assert_eq!(terminated_subscriptions.first().unwrap().0, peer_network_id);

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

        // Check the active subscriptions is still healthy
        check_subscription_connection(&mut subscription_manager, connected_peer, true);

        // Verify that the active subscription is still present
        assert!(subscription_manager
            .get_active_subscription_peers()
            .contains(&connected_peer));
    }

    #[tokio::test]
    async fn test_check_subscription_health_progress_stopped() {
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

        // Check the active subscription and verify that it is healthy
        check_subscription_progress(&mut subscription_manager, connected_peer, true);

        // Elapse time to simulate a DB progress error
        let mock_time_service = time_service.clone().into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_synced_version_timeout_ms + 1,
        ));

        // Check the active subscription and verify that it is unhealthy (the DB is not syncing)
        check_subscription_progress(&mut subscription_manager, connected_peer, false);

        // Terminate the subscription
        let terminated_subscriptions =
            terminate_any_unhealthy_subscriptions(&mut subscription_manager);
        assert_eq!(terminated_subscriptions.len(), 1);
        assert_eq!(terminated_subscriptions.first().unwrap().0, connected_peer);

        // Verify the active subscription is no longer present
        assert!(subscription_manager
            .get_active_subscription_peers()
            .is_empty());
    }

    #[tokio::test]
    async fn test_check_subscription_health_timeout() {
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

        // Check the active subscription and verify that it is healthy
        check_subscription_timeout(&mut subscription_manager, connected_peer, true);

        // Elapse time to simulate a timeout
        let mock_time_service = time_service.clone().into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_subscription_timeout_ms + 1,
        ));

        // Check the active subscription and verify that it is unhealthy (the subscription timed out)
        check_subscription_timeout(&mut subscription_manager, connected_peer, false);

        // Terminate the subscription
        let terminated_subscriptions =
            terminate_any_unhealthy_subscriptions(&mut subscription_manager);
        assert_eq!(terminated_subscriptions.len(), 1);
        assert_eq!(terminated_subscriptions.first().unwrap().0, connected_peer);

        // Verify the active subscription is no longer present
        assert!(subscription_manager
            .get_active_subscription_peers()
            .is_empty());
    }

    #[tokio::test]
    async fn test_check_subscription_health_suboptimal() {
        // Create a consensus observer config
        let consensus_observer_config = ConsensusObserverConfig {
            max_subscription_timeout_ms: 100_000_000, // Use a large value so that we don't time out
            max_concurrent_subscriptions: 1,          // Only allow one subscription
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
            create_peer_and_connection(network_id, peers_and_metadata.clone(), 1, None, true);

        // Create a new subscription to the suboptimal peer
        create_observer_subscription(
            &mut subscription_manager,
            consensus_observer_config,
            db_reader.clone(),
            suboptimal_peer,
            time_service.clone(),
        );

        // Check the active subscription and verify that it is healthy
        check_subscription_optimality(&mut subscription_manager, suboptimal_peer, true);

        // Elapse enough time to trigger the peer optimality check
        let mock_time_service = time_service.clone().into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_peer_change_interval_ms + 1,
        ));

        // Check the active subscription and verify that it is unhealthy (the peer is suboptimal)
        check_subscription_optimality(&mut subscription_manager, suboptimal_peer, false);

        // Elapse enough time to trigger the peer optimality check again
        let mock_time_service = time_service.clone().into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.subscription_refresh_interval_ms + 1,
        ));

        // Terminate the subscription
        let terminated_subscriptions =
            terminate_any_unhealthy_subscriptions(&mut subscription_manager);
        assert_eq!(terminated_subscriptions.len(), 1);
        assert_eq!(terminated_subscriptions.first().unwrap().0, suboptimal_peer);

        // Verify the active subscription is no longer present
        assert!(subscription_manager
            .get_active_subscription_peers()
            .is_empty());
    }

    #[tokio::test]
    async fn test_terminate_unhealthy_subscriptions() {
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

        // Create two new subscriptions
        let subscription_peer_1 =
            create_peer_and_connection(network_id, peers_and_metadata.clone(), 1, None, true);
        let subscription_peer_2 =
            create_peer_and_connection(network_id, peers_and_metadata.clone(), 1, None, true);
        for peer in &[subscription_peer_1, subscription_peer_2] {
            // Create the subscription
            create_observer_subscription(
                &mut subscription_manager,
                consensus_observer_config,
                db_reader.clone(),
                *peer,
                time_service.clone(),
            );
        }

        // Terminate any unhealthy subscriptions and verify that both subscriptions are still healthy
        let terminated_subscriptions =
            terminate_any_unhealthy_subscriptions(&mut subscription_manager);
        assert!(terminated_subscriptions.is_empty());
        assert_eq!(
            subscription_manager.get_active_subscription_peers().len(),
            2
        );

        // Create another subscription
        let subscription_peer_3 =
            create_peer_and_connection(network_id, peers_and_metadata.clone(), 1, None, true);
        create_observer_subscription(
            &mut subscription_manager,
            consensus_observer_config,
            db_reader.clone(),
            subscription_peer_3,
            TimeService::mock(), // Use a different time service (to avoid timeouts)
        );

        // Elapse time to simulate a timeout (on the first two subscriptions)
        let mock_time_service = time_service.into_mock();
        mock_time_service.advance(Duration::from_millis(
            consensus_observer_config.max_subscription_timeout_ms + 1,
        ));

        // Terminate the unhealthy subscriptions and verify the first two subscriptions were terminated
        let terminated_subscriptions =
            terminate_any_unhealthy_subscriptions(&mut subscription_manager);
        assert_eq!(terminated_subscriptions.len(), 2);
        assert_eq!(subscription_manager.get_active_subscription_peers(), vec![
            subscription_peer_3
        ]);

        // Verify that both subscriptions were terminated due to a timeout
        for (_, error) in terminated_subscriptions {
            assert_matches!(error, Error::SubscriptionTimeout(_));
        }
    }

    #[tokio::test]
    async fn test_unsubscribe_from_peer() {
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

        // Verify that no subscriptions are active
        assert!(subscription_manager
            .get_active_subscription_peers()
            .is_empty());

        // Create a new subscription
        let subscription_peer_1 = PeerNetworkId::random();
        create_observer_subscription(
            &mut subscription_manager,
            consensus_observer_config,
            db_reader.clone(),
            subscription_peer_1,
            TimeService::mock(),
        );

        // Verify the subscription is active
        assert!(subscription_manager
            .get_active_subscription_peers()
            .contains(&subscription_peer_1));

        // Create another subscription
        let subscription_peer_2 = PeerNetworkId::random();
        create_observer_subscription(
            &mut subscription_manager,
            consensus_observer_config,
            db_reader.clone(),
            subscription_peer_2,
            TimeService::mock(),
        );

        // Verify the second subscription is active
        assert!(subscription_manager
            .get_active_subscription_peers()
            .contains(&subscription_peer_2));

        // Unsubscribe from the first peer
        subscription_manager.unsubscribe_from_peer(subscription_peer_1);

        // Verify that the first subscription is no longer active
        assert!(!subscription_manager
            .get_active_subscription_peers()
            .contains(&subscription_peer_1));

        // Verify that only the second subscription is still active
        assert!(subscription_manager
            .get_active_subscription_peers()
            .contains(&subscription_peer_2));
        assert_eq!(
            subscription_manager.get_active_subscription_peers().len(),
            1
        );
    }

    #[tokio::test]
    async fn test_verify_message_for_subscription() {
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

        // Check that message verification fails (we have no active subscriptions)
        check_message_verification_result(
            &mut subscription_manager,
            PeerNetworkId::random(),
            false,
        );

        // Create a new subscription
        let subscription_peer = PeerNetworkId::random();
        create_observer_subscription(
            &mut subscription_manager,
            consensus_observer_config,
            db_reader.clone(),
            subscription_peer,
            TimeService::mock(),
        );

        // Check that message verification passes for the subscription
        check_message_verification_result(&mut subscription_manager, subscription_peer, true);

        // Create another subscription
        let second_subscription_peer = PeerNetworkId::random();
        create_observer_subscription(
            &mut subscription_manager,
            consensus_observer_config,
            db_reader.clone(),
            second_subscription_peer,
            TimeService::mock(),
        );

        // Check that message verification passes for the second subscription
        check_message_verification_result(
            &mut subscription_manager,
            second_subscription_peer,
            true,
        );

        // Check that message verification fails if the peer doesn't match either subscription
        check_message_verification_result(
            &mut subscription_manager,
            PeerNetworkId::random(),
            false,
        );
    }

    /// Checks the result of verifying a message from a given peer
    fn check_message_verification_result(
        subscription_manager: &mut SubscriptionManager,
        peer_network_id: PeerNetworkId,
        pass_verification: bool,
    ) {
        // Verify the message for the given peer
        let result = subscription_manager.verify_message_for_subscription(peer_network_id);

        // Ensure the result matches the expected value
        if pass_verification {
            assert!(result.is_ok());
        } else {
            assert_matches!(result, Err(Error::InvalidMessageError(_)));
        }
    }

    /// Checks the health of a subscription and verifies the connection status
    fn check_subscription_connection(
        subscription_manager: &mut SubscriptionManager,
        subscription_peer: PeerNetworkId,
        expect_connected: bool,
    ) {
        // Check the health of the subscription
        let connected_peers_and_metadata = subscription_manager.get_connected_peers_and_metadata();
        let result = subscription_manager
            .check_subscription_health(&connected_peers_and_metadata, subscription_peer);

        // Check the result based on the expected connection status
        if expect_connected {
            assert!(result.is_ok());
        } else {
            assert_matches!(result, Err(Error::SubscriptionDisconnected(_)));
        }
    }

    /// Checks the health of a subscription and verifies the optimality status
    fn check_subscription_optimality(
        subscription_manager: &mut SubscriptionManager,
        subscription_peer: PeerNetworkId,
        expect_optimal: bool,
    ) {
        // Check the health of the subscription
        let connected_peers_and_metadata = subscription_manager.get_connected_peers_and_metadata();
        let result = subscription_manager
            .check_subscription_health(&connected_peers_and_metadata, subscription_peer);

        // Check the result based on the expected optimality status
        if expect_optimal {
            assert!(result.is_ok());
        } else {
            assert_matches!(result, Err(Error::SubscriptionSuboptimal(_)));
        }
    }

    /// Checks the health of a subscription and verifies the progress status
    fn check_subscription_progress(
        subscription_manager: &mut SubscriptionManager,
        subscription_peer: PeerNetworkId,
        expect_progress: bool,
    ) {
        // Check the health of the subscription
        let connected_peers_and_metadata = subscription_manager.get_connected_peers_and_metadata();
        let result = subscription_manager
            .check_subscription_health(&connected_peers_and_metadata, subscription_peer);

        // Check the result based on the expected progress status
        if expect_progress {
            assert!(result.is_ok());
        } else {
            assert_matches!(result, Err(Error::SubscriptionProgressStopped(_)));
        }
    }

    /// Checks the health of a subscription and verifies the timeout status
    fn check_subscription_timeout(
        subscription_manager: &mut SubscriptionManager,
        subscription_peer: PeerNetworkId,
        expect_timeout: bool,
    ) {
        // Check the health of the subscription
        let connected_peers_and_metadata = subscription_manager.get_connected_peers_and_metadata();
        let result = subscription_manager
            .check_subscription_health(&connected_peers_and_metadata, subscription_peer);

        // Check the result based on the expected timeout status
        if expect_timeout {
            assert!(result.is_ok());
        } else {
            assert_matches!(result, Err(Error::SubscriptionTimeout(_)));
        }
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
        subscription_manager
            .active_observer_subscriptions
            .lock()
            .insert(subscription_peer, observer_subscription);
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

    /// A simple helper method that terminates any unhealthy subscriptions
    fn terminate_any_unhealthy_subscriptions(
        subscription_manager: &mut SubscriptionManager,
    ) -> Vec<(PeerNetworkId, Error)> {
        // Get the connected peers and metadata
        let connected_peers_and_metadata = subscription_manager.get_connected_peers_and_metadata();

        // Terminate any unhealthy subscriptions
        subscription_manager.terminate_unhealthy_subscriptions(&connected_peers_and_metadata)
    }
}
