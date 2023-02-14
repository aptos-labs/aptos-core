// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    PeerMonitoringServiceNetworkEvents, PeerMonitoringServiceServer, PEER_MONITORING_SERVER_VERSION,
};
use aptos_channels::aptos_channel;
use aptos_config::{
    config::{PeerMonitoringServiceConfig, PeerRole},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_logger::Level;
use aptos_netcore::transport::ConnectionOrigin;
use aptos_network::{
    application::{metadata::ConnectionState, storage::PeersAndMetadata},
    peer_manager::PeerManagerNotification,
    protocols::{
        network::NewNetworkEvents,
        rpc::InboundRpcRequest,
        wire::handshake::v1::{MessagingProtocolVersion, ProtocolId, ProtocolIdSet},
    },
    transport::{ConnectionId, ConnectionMetadata},
};
use aptos_peer_monitoring_service_types::{
    ConnectedPeersResponse, PeerMonitoringServiceError, PeerMonitoringServiceMessage,
    PeerMonitoringServiceRequest, PeerMonitoringServiceResponse, ServerProtocolVersionResponse,
};
use aptos_types::{network_address::NetworkAddress, PeerId};
use futures::channel::oneshot;
use std::{collections::HashMap, str::FromStr, sync::Arc};

#[tokio::test]
async fn test_get_server_protocol_version() {
    // Create the peer monitoring client and server
    let (mut mock_client, service, _) = MockClient::new();
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
async fn test_get_connected_peers() {
    // Create the peer monitoring client and server
    let (mut mock_client, service, peers_and_metadata) = MockClient::new();
    tokio::spawn(service.start());

    // Process a request to fetch the connected peers
    let request = PeerMonitoringServiceRequest::GetConnectedPeers;
    let response = mock_client.send_request(request).await.unwrap();

    // Verify the response is correct
    let expected_response = PeerMonitoringServiceResponse::ConnectedPeers(ConnectedPeersResponse {
        connected_peers: HashMap::new(),
    });
    assert_eq!(response, expected_response);

    // Update the connected peers with a new peer
    let peer_id = PeerId::random();
    let peer_network_id = PeerNetworkId::new(NetworkId::Validator, peer_id);
    let connection_metadata = ConnectionMetadata::new(
        peer_id,
        ConnectionId::default(),
        NetworkAddress::from_str("/ip4/127.0.0.1/tcp/8081").unwrap(),
        ConnectionOrigin::Inbound,
        MessagingProtocolVersion::V1,
        ProtocolIdSet::empty(),
        PeerRole::Unknown,
    );
    peers_and_metadata
        .insert_connection_metadata(peer_network_id, connection_metadata)
        .unwrap();

    // Process a request to fetch the connected peers
    let request = PeerMonitoringServiceRequest::GetConnectedPeers;
    let response = mock_client.send_request(request).await.unwrap();

    // Verify the response is correct
    let mut connected_peers = HashMap::new();
    connected_peers.insert(
        peer_network_id,
        peers_and_metadata
            .get_metadata_for_peer(peer_network_id)
            .unwrap(),
    );
    let expected_response =
        PeerMonitoringServiceResponse::ConnectedPeers(ConnectedPeersResponse { connected_peers });
    assert_eq!(response, expected_response);

    // Disconnect the peer
    peers_and_metadata
        .update_connection_state(peer_network_id, ConnectionState::Disconnected)
        .unwrap();

    // Process a request to fetch the connected peers
    let request = PeerMonitoringServiceRequest::GetConnectedPeers;
    let response = mock_client.send_request(request).await.unwrap();

    // Verify the response is correct
    let expected_response = PeerMonitoringServiceResponse::ConnectedPeers(ConnectedPeersResponse {
        connected_peers: HashMap::new(),
    });
    assert_eq!(response, expected_response);
}

/// A wrapper around the inbound network interface/channel for easily sending
/// mock client requests to a [`PeerMonitoringServiceServer`].
struct MockClient {
    peer_notification_sender: aptos_channel::Sender<(PeerId, ProtocolId), PeerManagerNotification>,
}

impl MockClient {
    fn new() -> (Self, PeerMonitoringServiceServer, Arc<PeersAndMetadata>) {
        initialize_logger();

        // Create the peer monitoring service event stream
        let peer_monitoring_service_config = PeerMonitoringServiceConfig::default();
        let network_endpoint_config = crate::network::peer_monitoring_service_network_config(
            peer_monitoring_service_config.clone(),
        )
        .inbound_queue_config;
        let (peer_notification_sender, peer_notification_receiver) =
            network_endpoint_config.build();
        let (_connection_notifications_receiver, connection_notifications_sender) =
            network_endpoint_config.build();
        let network_request_stream = PeerMonitoringServiceNetworkEvents::new(
            peer_notification_receiver,
            connection_notifications_sender,
        );

        // Create the peer monitoring server
        let peers_and_metadata = PeersAndMetadata::new(&[NetworkId::Validator]);
        let executor = tokio::runtime::Handle::current();
        let peer_monitoring_server = PeerMonitoringServiceServer::new(
            peer_monitoring_service_config,
            executor,
            network_request_stream,
            peers_and_metadata.clone(),
        );

        // Create the mock client
        let mock_client = Self {
            peer_notification_sender,
        };

        // Return the client and server
        (mock_client, peer_monitoring_server, peers_and_metadata)
    }

    async fn send_request(
        &mut self,
        request: PeerMonitoringServiceRequest,
    ) -> Result<PeerMonitoringServiceResponse, PeerMonitoringServiceError> {
        let peer_id = PeerId::ZERO;
        let protocol_id = ProtocolId::PeerMonitoringServiceRpc;

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
        self.peer_notification_sender
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
