// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    common::logging::{LogEntry, LogSchema},
    network::{
        observer_client::ConsensusObserverClient,
        observer_message::{
            ConsensusObserverMessage, ConsensusObserverRequest, ConsensusObserverResponse,
        },
    },
    observer::subscription::ConsensusObserverSubscription,
    publisher::consensus_publisher::ConsensusPublisher,
};
use aptos_config::{config::ConsensusObserverConfig, network_id::PeerNetworkId};
use aptos_logger::{error, info, warn};
use aptos_network::{
    application::{interface::NetworkClient, metadata::PeerMetadata},
    ProtocolId,
};
use aptos_storage_interface::DbReader;
use aptos_time_service::TimeService;
use ordered_float::OrderedFloat;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

// A useful constant for representing the maximum ping latency
const MAX_PING_LATENCY_SECS: f64 = 10_000.0;

/// Attempts to create the given number of new subscriptions
/// from the connected peers and metadata. Any active or unhealthy
/// subscriptions are excluded from the selection process.
pub async fn create_new_subscriptions(
    consensus_observer_config: ConsensusObserverConfig,
    consensus_observer_client: Arc<
        ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
    >,
    consensus_publisher: Option<Arc<ConsensusPublisher>>,
    db_reader: Arc<dyn DbReader>,
    time_service: TimeService,
    connected_peers_and_metadata: HashMap<PeerNetworkId, PeerMetadata>,
    num_subscriptions_to_create: usize,
    active_subscription_peers: Vec<PeerNetworkId>,
    unhealthy_subscription_peers: Vec<PeerNetworkId>,
) -> Vec<ConsensusObserverSubscription> {
    // Sort the potential peers for subscription requests
    let mut sorted_potential_peers = match sort_peers_for_subscriptions(
        connected_peers_and_metadata,
        unhealthy_subscription_peers,
        active_subscription_peers,
        consensus_publisher,
    ) {
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
    let mut created_subscriptions = vec![];
    for _ in 0..num_subscriptions_to_create {
        // If there are no peers left to subscribe to, return early
        if sorted_potential_peers.is_empty() {
            info!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "There are no more potential peers to subscribe to! \
                    Num created subscriptions: {:?}",
                    created_subscriptions.len()
                ))
            );
            break;
        }

        // Attempt to create a new subscription
        let (observer_subscription, failed_subscription_peers) = create_single_subscription(
            consensus_observer_config,
            consensus_observer_client.clone(),
            db_reader.clone(),
            sorted_potential_peers.clone(),
            time_service.clone(),
        )
        .await;

        // Remove the failed peers from the sorted list
        sorted_potential_peers.retain(|peer| !failed_subscription_peers.contains(peer));

        // Process a successful subscription creation
        if let Some(observer_subscription) = observer_subscription {
            // Remove the peer from the sorted list (for the next selection)
            sorted_potential_peers
                .retain(|peer| *peer != observer_subscription.get_peer_network_id());

            // Add the newly created subscription to the subscription list
            created_subscriptions.push(observer_subscription);
        }
    }

    // Return the list of created subscriptions
    created_subscriptions
}

/// Attempts to create a new subscription to a single peer from the
/// sorted list of potential peers. If successful, the new subscription
/// is returned, alongside any peers with failed attempts.
async fn create_single_subscription(
    consensus_observer_config: ConsensusObserverConfig,
    consensus_observer_client: Arc<
        ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
    >,
    db_reader: Arc<dyn DbReader>,
    sorted_potential_peers: Vec<PeerNetworkId>,
    time_service: TimeService,
) -> (Option<ConsensusObserverSubscription>, Vec<PeerNetworkId>) {
    let mut peers_with_failed_attempts = vec![];
    for potential_peer in sorted_potential_peers {
        // Log the subscription attempt
        info!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Attempting to subscribe to potential peer: {}!",
                potential_peer
            ))
        );

        // Send a subscription request to the peer and wait for the response
        let subscription_request = ConsensusObserverRequest::Subscribe;
        let request_timeout_ms = consensus_observer_config.network_request_timeout_ms;
        let response = consensus_observer_client
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
                    consensus_observer_config,
                    db_reader.clone(),
                    potential_peer,
                    time_service.clone(),
                );

                // Return the successful subscription
                return (Some(subscription), peers_with_failed_attempts);
            },
            Ok(response) => {
                // We received an invalid response
                warn!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Got unexpected response type for subscription request: {:?}",
                        response.get_label()
                    ))
                );

                // Add the peer to the list of failed attempts
                peers_with_failed_attempts.push(potential_peer);
            },
            Err(error) => {
                // We encountered an error while sending the request
                error!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Failed to send subscription request to peer: {}! Error: {:?}",
                        potential_peer, error
                    ))
                );

                // Add the peer to the list of failed attempts
                peers_with_failed_attempts.push(potential_peer);
            },
        }
    }

    // We failed to create a new subscription
    (None, peers_with_failed_attempts)
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

/// Produces a list of sorted peers to service the subscription requests.
/// Any active or unhealthy subscriptions are excluded from the selection process.
/// Likewise, any peers currently subscribed to us are also excluded.
fn sort_peers_for_subscriptions(
    mut connected_peers_and_metadata: HashMap<PeerNetworkId, PeerMetadata>,
    active_subscription_peers: Vec<PeerNetworkId>,
    unhealthy_subscription_peers: Vec<PeerNetworkId>,
    consensus_publisher: Option<Arc<ConsensusPublisher>>,
) -> Option<Vec<PeerNetworkId>> {
    // Remove any peers we're already subscribed to
    for active_subscription_peer in active_subscription_peers {
        let _ = connected_peers_and_metadata.remove(&active_subscription_peer);
    }

    // Remove any unhealthy subscription peers
    for unhealthy_peer in unhealthy_subscription_peers {
        let _ = connected_peers_and_metadata.remove(&unhealthy_peer);
    }

    // Remove any peers that are currently subscribed to us
    if let Some(consensus_publisher) = consensus_publisher {
        for peer_network_id in consensus_publisher.get_active_subscribers() {
            let _ = connected_peers_and_metadata.remove(&peer_network_id);
        }
    }

    // Sort the peers by subscription optimality
    let sorted_peers = sort_peers_by_subscription_optimality(&connected_peers_and_metadata);

    // Return the sorted peers
    Some(sorted_peers)
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
mod tests {
    use super::*;
    use aptos_config::{config::PeerRole, network_id::NetworkId};
    use aptos_netcore::transport::ConnectionOrigin;
    use aptos_network::{
        application::storage::PeersAndMetadata,
        protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolIdSet},
        transport::{ConnectionId, ConnectionMetadata},
    };
    use aptos_peer_monitoring_service_types::{
        response::NetworkInformationResponse, PeerMonitoringMetadata,
    };
    use aptos_types::{network_address::NetworkAddress, PeerId};
    use maplit::hashmap;
    use std::collections::HashSet;

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

    #[tokio::test]
    async fn test_sort_peers_for_subscriptions() {
        // Create a consensus observer client
        let network_ids = &[NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
        let (peers_and_metadata, consensus_observer_client) =
            create_consensus_observer_client(network_ids);

        // Create a consensus publisher
        let consensus_observer_config = ConsensusObserverConfig::default();
        let (consensus_publisher, _) =
            ConsensusPublisher::new(consensus_observer_config, consensus_observer_client.clone());
        let consensus_publisher = Arc::new(consensus_publisher);

        // Sort the peers and verify that no peers are returned
        let sorted_peers = sort_subscription_peers(
            consensus_publisher.clone(),
            peers_and_metadata.clone(),
            vec![],
            vec![],
        );
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

        // Sort the peers and verify the ordering (according to distance)
        let sorted_peers = sort_subscription_peers(
            consensus_publisher.clone(),
            peers_and_metadata.clone(),
            vec![],
            vec![],
        );
        assert_eq!(sorted_peers[0].network_id(), NetworkId::Validator);
        assert_eq!(sorted_peers[1].network_id(), NetworkId::Vfn);
        assert_eq!(sorted_peers[2].network_id(), NetworkId::Public);
        assert_eq!(sorted_peers.len(), 3);

        // Sort the peers, but mark the validator as unhealthy (so it's ignored)
        let sorted_peer_subset = sort_subscription_peers(
            consensus_publisher.clone(),
            peers_and_metadata.clone(),
            vec![],
            vec![sorted_peers[0]],
        );
        assert_eq!(sorted_peer_subset[0].network_id(), NetworkId::Vfn);
        assert_eq!(sorted_peer_subset[1].network_id(), NetworkId::Public);
        assert_eq!(sorted_peer_subset.len(), 2);

        // Sort the peers, but mark the VFN and validator as active subscriptions (so they're ignored)
        let sorted_peer_subset = sort_subscription_peers(
            consensus_publisher.clone(),
            peers_and_metadata.clone(),
            vec![sorted_peers[0], sorted_peers[1]],
            vec![],
        );
        assert_eq!(sorted_peer_subset[0].network_id(), NetworkId::Public);
        assert_eq!(sorted_peer_subset.len(), 1);

        // Create a consensus publisher with the PFN as an active subscriber
        let consensus_publisher_with_subscribers =
            Arc::new(ConsensusPublisher::new_with_active_subscribers(
                consensus_observer_config,
                consensus_observer_client.clone(),
                HashSet::from_iter(vec![sorted_peers[2]]),
            ));

        // Sort the peers, and verify the PFN is ignored (since it's an active subscriber)
        let sorted_peer_subset = sort_subscription_peers(
            consensus_publisher_with_subscribers,
            peers_and_metadata.clone(),
            vec![],
            vec![],
        );
        assert_eq!(sorted_peer_subset[0].network_id(), NetworkId::Validator);
        assert_eq!(sorted_peer_subset[1].network_id(), NetworkId::Vfn);
        assert_eq!(sorted_peer_subset.len(), 2);

        // Remove all the peers and verify that no peers are returned upon sorting
        for peer_network_id in sorted_peers {
            remove_peer_and_connection(peers_and_metadata.clone(), peer_network_id);
        }
        let sorted_peers = sort_subscription_peers(
            consensus_publisher.clone(),
            peers_and_metadata.clone(),
            vec![],
            vec![],
        );
        assert!(sorted_peers.is_empty());

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

        // Sort the peers and verify the ordering (according to latency)
        let sorted_peers = sort_subscription_peers(
            consensus_publisher,
            peers_and_metadata.clone(),
            vec![],
            vec![],
        );
        let expected_peers = validator_peers.into_iter().rev().collect::<Vec<_>>();
        assert_eq!(sorted_peers, expected_peers);
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

    /// A simple helper method that sorts the given peers for a subscription
    fn sort_subscription_peers(
        consensus_publisher: Arc<ConsensusPublisher>,
        peers_and_metadata: Arc<PeersAndMetadata>,
        active_subscription_peers: Vec<PeerNetworkId>,
        unhealthy_subscription_peers: Vec<PeerNetworkId>,
    ) -> Vec<PeerNetworkId> {
        // Get the connected peers and metadata
        let connected_peers_and_metadata = peers_and_metadata
            .get_connected_peers_and_metadata()
            .unwrap();

        // Sort the peers for subscription requests
        sort_peers_for_subscriptions(
            connected_peers_and_metadata,
            unhealthy_subscription_peers,
            active_subscription_peers,
            Some(consensus_publisher),
        )
        .unwrap()
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
