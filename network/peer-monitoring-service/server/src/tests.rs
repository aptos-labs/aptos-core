// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    metrics, PeerMonitoringServiceNetworkEvents, PeerMonitoringServiceServer,
    MAX_DISTANCE_FROM_VALIDATORS, PEER_MONITORING_SERVER_VERSION,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::{
    config::{BaseConfig, NodeConfig, PeerMonitoringServiceConfig, PeerRole, RoleType},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_logger::Level;
use aptos_netcore::transport::ConnectionOrigin;
use aptos_network::{
    application::{
        interface::NetworkServiceEvents,
        metadata::{ConnectionState, PeerMetadata, PeerMonitoringMetadata},
        storage::PeersAndMetadata,
    },
    peer_manager::PeerManagerNotification,
    protocols::{
        network::{NetworkEvents, NewNetworkEvents},
        rpc::InboundRpcRequest,
        wire::handshake::v1::{MessagingProtocolVersion, ProtocolId, ProtocolIdSet},
    },
    transport::{ConnectionId, ConnectionMetadata},
};
use aptos_peer_monitoring_service_types::{
    LatencyPingRequest, NetworkInformationResponse, PeerMonitoringServiceError,
    PeerMonitoringServiceMessage, PeerMonitoringServiceRequest, PeerMonitoringServiceResponse,
    ServerProtocolVersionResponse,
};
use aptos_time_service::{MockTimeService, TimeService};
use aptos_types::{account_address::AccountAddress, network_address::NetworkAddress, PeerId};
use futures::channel::oneshot;
use maplit::hashmap;
use rand::{rngs::OsRng, Rng};
use std::{collections::HashMap, str::FromStr, sync::Arc};

// Useful test constants
const LOCAL_HOST_NET_ADDR: &str = "/ip4/127.0.0.1/tcp/8081";

#[tokio::test]
async fn test_get_server_protocol_version() {
    // Create the peer monitoring client and server
    let (mut mock_client, service, _, _) = MockClient::new(None, None);
    tokio::spawn(service.start());

    // Process a request to fetch the protocol version
    let request = PeerMonitoringServiceRequest::GetServerProtocolVersion;
    let response = mock_client.send_request(request).await.unwrap();

    // Verify the response is correct
    let expected_response =
        PeerMonitoringServiceResponse::ServerProtocolVersion(ServerProtocolVersionResponse {
            version: PEER_MONITORING_SERVER_VERSION,
        });
    assert_eq!(response, expected_response);
}

#[tokio::test]
async fn test_get_network_information_fullnode() {
    // Create the peer monitoring client and server
    let base_config = BaseConfig {
        role: RoleType::FullNode, // The server is a fullnode
        ..Default::default()
    };
    let (mut mock_client, service, _, peers_and_metadata) =
        MockClient::new(Some(base_config), None);
    tokio::spawn(service.start());

    // Process a client request to fetch the network information and verify an empty response
    verify_network_information(
        &mut mock_client,
        HashMap::new(),
        MAX_DISTANCE_FROM_VALIDATORS,
    )
    .await;

    // Connect a new peer to the fullnode
    let peer_id_1 = PeerId::random();
    let peer_network_id_1 = PeerNetworkId::new(NetworkId::Public, peer_id_1);
    let mut connection_metadata_1 = create_connection_metadata(peer_id_1);
    peers_and_metadata
        .insert_connection_metadata(peer_network_id_1, connection_metadata_1.clone())
        .unwrap();

    // Process a client request to fetch the network information and verify the response
    verify_network_information(
        &mut mock_client,
        hashmap! {peer_network_id_1 => PeerMetadata::new(connection_metadata_1.clone())},
        MAX_DISTANCE_FROM_VALIDATORS,
    )
    .await;

    // Update the peer monitoring metadata for peer 1
    let peer_distance_1 = MAX_DISTANCE_FROM_VALIDATORS; // Peer 1 is not connected to anyone else
    let peer_monitoring_metadata_1 = PeerMonitoringMetadata::new(None, None, Some(peer_distance_1));
    peers_and_metadata
        .update_peer_monitoring_metadata(peer_network_id_1, peer_monitoring_metadata_1.clone())
        .unwrap();

    // Process a client request to fetch the network information and verify the response
    let peer_metadata_1 =
        PeerMetadata::new_for_test(connection_metadata_1.clone(), peer_monitoring_metadata_1);
    verify_network_information(
        &mut mock_client,
        hashmap! {peer_network_id_1 => peer_metadata_1.clone()},
        MAX_DISTANCE_FROM_VALIDATORS,
    )
    .await;

    // Update the peer monitoring metadata and connection metadata for peer 1
    let peer_distance_1 = 2; // Peer 1 now has other connections
    let peer_monitoring_metadata_1 = PeerMonitoringMetadata::new(None, None, Some(peer_distance_1));
    peers_and_metadata
        .update_peer_monitoring_metadata(peer_network_id_1, peer_monitoring_metadata_1.clone())
        .unwrap();
    connection_metadata_1.connection_id = ConnectionId::from(101);
    peers_and_metadata
        .insert_connection_metadata(peer_network_id_1, connection_metadata_1.clone())
        .unwrap();

    // Process a client request to fetch the network information and verify the response
    let peer_metadata_1 =
        PeerMetadata::new_for_test(connection_metadata_1.clone(), peer_monitoring_metadata_1);
    verify_network_information(
        &mut mock_client,
        hashmap! {peer_network_id_1 => peer_metadata_1.clone()},
        peer_distance_1 + 1,
    )
    .await;

    // Connect another peer to the fullnode
    let peer_id_2 = PeerId::random();
    let peer_network_id_2 = PeerNetworkId::new(NetworkId::Validator, peer_id_2);
    let peer_distance_2 = 0; // The peer is a validator
    let connection_metadata_2 = create_connection_metadata(peer_id_2);
    let peer_monitoring_metadata_2 = PeerMonitoringMetadata::new(None, None, Some(peer_distance_2));
    peers_and_metadata
        .insert_connection_metadata(peer_network_id_2, connection_metadata_2.clone())
        .unwrap();
    peers_and_metadata
        .update_peer_monitoring_metadata(peer_network_id_2, peer_monitoring_metadata_2.clone())
        .unwrap();

    // Process a client request to fetch the network information and verify the response
    let peer_metadata_2 =
        PeerMetadata::new_for_test(connection_metadata_2.clone(), peer_monitoring_metadata_2);
    verify_network_information(
        &mut mock_client,
        hashmap! {peer_network_id_1 => peer_metadata_1.clone(), peer_network_id_2 => peer_metadata_2},
        peer_distance_2 + 1,
    )
    .await;

    // Disconnect peer 2
    peers_and_metadata
        .update_connection_state(peer_network_id_2, ConnectionState::Disconnected)
        .unwrap();

    // Process a request to fetch the network information and verify the response
    verify_network_information(
        &mut mock_client,
        hashmap! {peer_network_id_1 => peer_metadata_1},
        peer_distance_1 + 1,
    )
    .await;
}

#[tokio::test]
async fn test_get_network_information_validator() {
    // Create the peer monitoring client and server
    let base_config = BaseConfig {
        role: RoleType::Validator, // The server is a validator
        ..Default::default()
    };
    let (mut mock_client, service, _, peers_and_metadata) =
        MockClient::new(Some(base_config), None);
    tokio::spawn(service.start());

    // Process a client request to fetch the network information and verify distance is 0
    verify_network_information(&mut mock_client, HashMap::new(), 0).await;

    // Connect a new peer to the validator (another validator)
    let peer_id_1 = PeerId::random();
    let peer_network_id_1 = PeerNetworkId::new(NetworkId::Validator, peer_id_1);
    let connection_metadata_1 = create_connection_metadata(peer_id_1);
    peers_and_metadata
        .insert_connection_metadata(peer_network_id_1, connection_metadata_1.clone())
        .unwrap();

    // Process a client request to fetch the network information and verify the response
    verify_network_information(
        &mut mock_client,
        hashmap! {peer_network_id_1 => PeerMetadata::new(connection_metadata_1.clone())},
        0,
    )
    .await;

    // Update the peer monitoring metadata for peer 1
    let peer_distance_1 = 0; // Peer 1 now has other connections
    let peer_monitoring_metadata_1 = PeerMonitoringMetadata::new(None, None, Some(peer_distance_1));
    peers_and_metadata
        .update_peer_monitoring_metadata(peer_network_id_1, peer_monitoring_metadata_1.clone())
        .unwrap();

    // Process a client request to fetch the network information and verify the response
    let peer_metadata_1 =
        PeerMetadata::new_for_test(connection_metadata_1.clone(), peer_monitoring_metadata_1);
    verify_network_information(
        &mut mock_client,
        hashmap! {peer_network_id_1 => peer_metadata_1.clone()},
        0,
    )
    .await;

    // Connect another peer to the validator
    let peer_id_2 = PeerId::random();
    let peer_network_id_2 = PeerNetworkId::new(NetworkId::Vfn, peer_id_2);
    let peer_distance_2 = 1; // The peer is a VFN
    let connection_metadata_2 = create_connection_metadata(peer_id_2);
    let peer_monitoring_metadata_2 = PeerMonitoringMetadata::new(None, None, Some(peer_distance_2));
    peers_and_metadata
        .insert_connection_metadata(peer_network_id_2, connection_metadata_2.clone())
        .unwrap();
    peers_and_metadata
        .update_peer_monitoring_metadata(peer_network_id_2, peer_monitoring_metadata_2.clone())
        .unwrap();

    // Process a client request to fetch the network information and verify the response
    let peer_metadata_2 =
        PeerMetadata::new_for_test(connection_metadata_2.clone(), peer_monitoring_metadata_2);
    verify_network_information(
        &mut mock_client,
        hashmap! {peer_network_id_1 => peer_metadata_1.clone(), peer_network_id_2 => peer_metadata_2},
        0,
    )
        .await;

    // Disconnect peer 2
    peers_and_metadata
        .update_connection_state(peer_network_id_2, ConnectionState::Disconnected)
        .unwrap();

    // Process a request to fetch the network information and verify the response
    verify_network_information(
        &mut mock_client,
        hashmap! {peer_network_id_1 => peer_metadata_1},
        0,
    )
    .await;
}

#[tokio::test]
async fn test_latency_ping_request() {
    // Create the peer monitoring client and server
    let (mut mock_client, service, _, _) = MockClient::new(None, None);
    tokio::spawn(service.start());

    // Process several requests to perform latency pings
    for i in 0..10 {
        let request =
            PeerMonitoringServiceRequest::LatencyPing(LatencyPingRequest { ping_counter: i });
        let response = mock_client.send_request(request).await.unwrap();
        match response {
            PeerMonitoringServiceResponse::LatencyPing(latecy_ping_response) => {
                assert_eq!(latecy_ping_response.ping_counter, i);
            },
            _ => panic!("Expected latency ping response but got: {:?}", response),
        }
    }
}

/// A simple utility function to create a new connection metadata for tests
fn create_connection_metadata(peer_id: AccountAddress) -> ConnectionMetadata {
    ConnectionMetadata::new(
        peer_id,
        ConnectionId::default(),
        NetworkAddress::from_str(LOCAL_HOST_NET_ADDR).unwrap(),
        ConnectionOrigin::Inbound,
        MessagingProtocolVersion::V1,
        ProtocolIdSet::empty(),
        PeerRole::Unknown,
    )
}

/// A simple utility function that sends a request for network info using the given
/// client, and verifies the response is correct.
async fn verify_network_information(
    client: &mut MockClient,
    expected_peers_and_metadata: HashMap<PeerNetworkId, PeerMetadata>,
    expected_distance_from_validators: u64,
) {
    // Send a request to fetch the network information
    let request = PeerMonitoringServiceRequest::GetNetworkInformation;
    let response = client.send_request(request).await.unwrap();

    // Verify the response is correct
    let expected_response =
        PeerMonitoringServiceResponse::NetworkInformation(NetworkInformationResponse {
            connected_peers_and_metadata: expected_peers_and_metadata,
            distance_from_validators: expected_distance_from_validators,
        });
    assert_eq!(response, expected_response);
}

// A wrapper around the inbound network interface/channel for easily sending
/// mock client requests to a peer monitoring service server.
struct MockClient {
    peer_manager_notifiers:
        HashMap<NetworkId, aptos_channel::Sender<(PeerId, ProtocolId), PeerManagerNotification>>,
}

impl MockClient {
    fn new(
        base_config: Option<BaseConfig>,
        peer_monitoring_config: Option<PeerMonitoringServiceConfig>,
    ) -> (
        Self,
        PeerMonitoringServiceServer,
        MockTimeService,
        Arc<PeersAndMetadata>,
    ) {
        initialize_logger();

        // Create the node config
        let base_config = base_config.unwrap_or_default();
        let peer_monitoring_config = peer_monitoring_config.unwrap_or_default();
        let node_config = NodeConfig {
            base: base_config,
            peer_monitoring_service: peer_monitoring_config.clone(),
            ..Default::default()
        };

        // Setup the networks and the network events
        let network_ids = vec![NetworkId::Validator, NetworkId::Vfn, NetworkId::Public];
        let peers_and_metadata = PeersAndMetadata::new(&network_ids);
        let mut network_and_events = HashMap::new();
        let mut peer_manager_notifiers = HashMap::new();
        for network_id in network_ids {
            let queue_cfg = aptos_channel::Config::new(
                peer_monitoring_config.max_network_channel_size as usize,
            )
            .queue_style(QueueStyle::FIFO)
            .counters(&metrics::PENDING_PEER_MONITORING_SERVER_NETWORK_EVENTS);
            let (peer_manager_notifier, peer_manager_notification_receiver) = queue_cfg.build();
            let (_, connection_notification_receiver) = queue_cfg.build();

            let network_events = NetworkEvents::new(
                peer_manager_notification_receiver,
                connection_notification_receiver,
            );
            network_and_events.insert(network_id, network_events);
            peer_manager_notifiers.insert(network_id, peer_manager_notifier);
        }
        let peer_monitoring_network_events =
            PeerMonitoringServiceNetworkEvents::new(NetworkServiceEvents::new(network_and_events));

        // Create the storage service
        let executor = tokio::runtime::Handle::current();
        let mock_time_service = TimeService::mock();
        let peer_monitoring_server = PeerMonitoringServiceServer::new(
            node_config,
            executor,
            peer_monitoring_network_events,
            peers_and_metadata.clone(),
        );

        // Create the client
        let mock_client = Self {
            peer_manager_notifiers,
        };

        (
            mock_client,
            peer_monitoring_server,
            mock_time_service.into_mock(),
            peers_and_metadata,
        )
    }

    /// Sends the specified request and returns the response from the server
    async fn send_request(
        &mut self,
        request: PeerMonitoringServiceRequest,
    ) -> Result<PeerMonitoringServiceResponse, PeerMonitoringServiceError> {
        let peer_id = PeerId::random();
        let protocol_id = ProtocolId::PeerMonitoringServiceRpc;
        let network_id = get_random_network_id();

        // Create an inbound RPC request
        let request_data = protocol_id
            .to_bytes(&PeerMonitoringServiceMessage::Request(request))
            .unwrap();
        let (request_sender, request_receiver) = oneshot::channel();
        let inbound_rpc = InboundRpcRequest {
            protocol_id,
            data: request_data.into(),
            res_tx: request_sender,
        };
        let request_notification = PeerManagerNotification::RecvRpc(peer_id, inbound_rpc);

        // Send the request to the peer monitoring service
        self.peer_manager_notifiers
            .get(&network_id)
            .unwrap()
            .push((peer_id, protocol_id), request_notification)
            .unwrap();

        // Wait for the response from the peer monitoring service
        let response_data = request_receiver.await.unwrap().unwrap();
        let response = protocol_id
            .from_bytes::<PeerMonitoringServiceMessage>(&response_data)
            .unwrap();
        match response {
            PeerMonitoringServiceMessage::Response(response) => response,
            _ => panic!("Unexpected response message: {:?}", response),
        }
    }
}

/// Initializes the Aptos logger for tests
pub fn initialize_logger() {
    aptos_logger::Logger::builder()
        .is_async(false)
        .level(Level::Debug)
        .build();
}

/// Returns a random network ID
fn get_random_network_id() -> NetworkId {
    let mut rng = OsRng;
    let random_number: u8 = rng.gen();
    match random_number % 3 {
        0 => NetworkId::Validator,
        1 => NetworkId::Vfn,
        2 => NetworkId::Public,
        num => panic!("This shouldn't be possible! Got num: {:?}", num),
    }
}
