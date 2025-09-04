// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics,
    peer_states::{key_value::StateValueInterface, request_tracker::RequestTracker},
    Error, LogEntry, LogEvent, LogSchema,
};
use velor_config::{
    config::{BaseConfig, NetworkMonitoringConfig, NodeConfig, RoleType},
    network_id::PeerNetworkId,
};
use velor_infallible::RwLock;
use velor_logger::warn;
use velor_network::application::metadata::PeerMetadata;
use velor_peer_monitoring_service_types::{
    request::PeerMonitoringServiceRequest,
    response::{NetworkInformationResponse, PeerMonitoringServiceResponse},
    MAX_DISTANCE_FROM_VALIDATORS,
};
use velor_time_service::TimeService;
use std::{
    fmt,
    fmt::{Display, Formatter},
    sync::Arc,
};

/// A simple container that holds a single peer's network info
#[derive(Clone, Debug)]
pub struct NetworkInfoState {
    base_config: BaseConfig, // The base config of this node
    network_monitoring_config: NetworkMonitoringConfig, // The config for network monitoring
    recorded_network_info_response: Option<NetworkInformationResponse>, // The last network info response
    request_tracker: Arc<RwLock<RequestTracker>>, // The request tracker for network info requests
}

impl NetworkInfoState {
    pub fn new(node_config: NodeConfig, time_service: TimeService) -> Self {
        let base_config = node_config.base;
        let network_monitoring_config = node_config.peer_monitoring_service.network_monitoring;
        let request_tracker = RequestTracker::new(
            network_monitoring_config.network_info_request_interval_ms,
            time_service,
        );

        Self {
            base_config,
            network_monitoring_config,
            recorded_network_info_response: None,
            request_tracker: Arc::new(RwLock::new(request_tracker)),
        }
    }

    /// Records the new network info response for the peer
    pub fn record_network_info_response(
        &mut self,
        network_info_response: NetworkInformationResponse,
    ) {
        // Update the request tracker with a successful response
        self.request_tracker.write().record_response_success();

        // Save the network info
        self.recorded_network_info_response = Some(network_info_response);
    }

    /// Handles a request failure for the specified peer
    fn handle_request_failure(&self) {
        self.request_tracker.write().record_response_failure();
    }

    /// Returns the latest network info response
    pub fn get_latest_network_info_response(&self) -> Option<NetworkInformationResponse> {
        self.recorded_network_info_response.clone()
    }
}

impl StateValueInterface for NetworkInfoState {
    fn create_monitoring_service_request(&mut self) -> PeerMonitoringServiceRequest {
        PeerMonitoringServiceRequest::GetNetworkInformation
    }

    fn get_request_timeout_ms(&self) -> u64 {
        self.network_monitoring_config
            .network_info_request_timeout_ms
    }

    fn get_request_tracker(&self) -> Arc<RwLock<RequestTracker>> {
        self.request_tracker.clone()
    }

    fn handle_monitoring_service_response(
        &mut self,
        peer_network_id: &PeerNetworkId,
        peer_metadata: PeerMetadata,
        _monitoring_service_request: PeerMonitoringServiceRequest,
        monitoring_service_response: PeerMonitoringServiceResponse,
        _response_time_secs: f64,
    ) {
        // Verify the response type is valid
        let network_info_response = match monitoring_service_response {
            PeerMonitoringServiceResponse::NetworkInformation(network_information_response) => {
                network_information_response
            },
            _ => {
                warn!(LogSchema::new(LogEntry::NetworkInfoRequest)
                    .event(LogEvent::ResponseError)
                    .peer(peer_network_id)
                    .message(
                        "An unexpected response was received instead of a network info response!"
                    ));
                self.handle_request_failure();
                return;
            },
        };

        // Sanity check the response depth from the peer metadata
        let network_id = peer_network_id.network_id();
        let is_valid_depth = match network_info_response.distance_from_validators {
            0 => {
                // Verify the peer is a validator and has the correct network id
                let peer_is_validator = peer_metadata.get_connection_metadata().role.is_validator();
                let peer_has_correct_network = match self.base_config.role {
                    RoleType::Validator => network_id.is_validator_network(), // We're a validator
                    RoleType::FullNode => network_id.is_vfn_network(),        // We're a VFN
                };
                peer_is_validator && peer_has_correct_network
            },
            1 => {
                // Verify the peer is a VFN and has the correct network id
                let peer_is_vfn = peer_metadata.get_connection_metadata().role.is_vfn();
                let peer_has_correct_network = match self.base_config.role {
                    RoleType::Validator => network_id.is_vfn_network(), // We're a validator
                    RoleType::FullNode => network_id.is_public_network(), // We're a VFN or PFN
                };
                peer_is_vfn && peer_has_correct_network
            },
            distance_from_validators => {
                // The distance must be less than or equal to the max
                distance_from_validators <= MAX_DISTANCE_FROM_VALIDATORS
            },
        };

        // If the depth did not pass our sanity checks, handle a failure
        if !is_valid_depth {
            warn!(LogSchema::new(LogEntry::NetworkInfoRequest)
                .event(LogEvent::InvalidResponse)
                .peer(peer_network_id)
                .message(&format!(
                    "Peer returned invalid depth from validators: {}",
                    network_info_response.distance_from_validators
                )));
            self.handle_request_failure();
            return;
        }

        // Store the new latency ping result
        self.record_network_info_response(network_info_response);
    }

    fn handle_monitoring_service_response_error(
        &mut self,
        peer_network_id: &PeerNetworkId,
        error: Error,
    ) {
        // Handle the failure
        self.handle_request_failure();

        // Log the error
        warn!(LogSchema::new(LogEntry::NetworkInfoRequest)
            .event(LogEvent::ResponseError)
            .message("Error encountered when requesting network information from the peer!")
            .peer(peer_network_id)
            .error(&error));
    }

    fn update_peer_state_metrics(&self, peer_network_id: &PeerNetworkId) {
        if let Some(network_info_response) = self.get_latest_network_info_response() {
            // Update the distance from the validators metric
            let distance_from_validators = network_info_response.distance_from_validators;
            metrics::observe_value(
                &metrics::DISTANCE_FROM_VALIDATORS,
                peer_network_id,
                distance_from_validators as f64,
            );

            // Update the number of connected peers metric
            let num_connected_peers = network_info_response.connected_peers.len();
            metrics::observe_value(
                &metrics::NUM_CONNECTED_PEERS,
                peer_network_id,
                num_connected_peers as f64,
            );
        }
    }
}

impl Display for NetworkInfoState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NetworkInfoState {{ recorded_network_info_response: {:?} }}",
            self.recorded_network_info_response
        )
    }
}

#[cfg(test)]
mod test {
    use crate::peer_states::{key_value::StateValueInterface, network_info::NetworkInfoState};
    use velor_config::{
        config::{BaseConfig, NodeConfig, PeerRole, RoleType},
        network_id::{NetworkId, PeerNetworkId},
    };
    use velor_netcore::transport::ConnectionOrigin;
    use velor_network::{
        application::metadata::PeerMetadata,
        protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolIdSet},
        transport::{ConnectionId, ConnectionMetadata},
    };
    use velor_peer_monitoring_service_types::{
        request::PeerMonitoringServiceRequest,
        response::{NetworkInformationResponse, PeerMonitoringServiceResponse},
    };
    use velor_time_service::TimeService;
    use velor_types::{network_address::NetworkAddress, PeerId};
    use std::str::FromStr;

    // Useful test constants
    const TEST_NETWORK_ADDRESS: &str = "/ip4/127.0.0.1/tcp/8081";

    #[test]
    fn test_sanity_check_distance_validator() {
        // Create the network info state for a validator
        let mut network_info_state = create_network_info_state(RoleType::Validator);

        // Verify there is no latest network info response
        verify_empty_network_response(&network_info_state);

        // Attempt to store a network response with an invalid depth of
        // 0 (the peer is a VFN, not a validator).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Vfn,
            PeerRole::ValidatorFullNode,
            0,
            None,
        );

        // Attempt to store a network response with an invalid depth of
        // 1 (the peer is a validator, not a VFN).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Validator,
            PeerRole::Validator,
            1,
            None,
        );

        // Attempt to store a network response with a valid depth of
        // 3 (the peer is a validator that is disconnected from the set).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Validator,
            PeerRole::Validator,
            3,
            Some(3),
        );

        // Attempt to store a network response with a valid depth of
        // 10 (the peer is a VFN that has poor connections).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Vfn,
            PeerRole::ValidatorFullNode,
            10,
            Some(10),
        );

        // Attempt to store a network response with a valid depth of
        // 1 (the peer is a VFN).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Vfn,
            PeerRole::ValidatorFullNode,
            1,
            Some(1),
        );

        // Attempt to store a network response with a valid depth of
        // 0 (the peer is a validator).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Validator,
            PeerRole::Validator,
            0,
            Some(0),
        );
    }

    #[test]
    fn test_sanity_check_distance_vfn() {
        // Create the network info state for a VFN
        let mut network_info_state = create_network_info_state(RoleType::FullNode);

        // Verify there is no latest network info response
        verify_empty_network_response(&network_info_state);

        // Attempt to store a network response with an invalid depth of
        // 1 (the peer is a validator).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Validator,
            PeerRole::Validator,
            1,
            None,
        );

        // Attempt to store a network response with an invalid depth of
        // 0 (the peer is a PFN, not a validator).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Public,
            PeerRole::Unknown,
            0,
            None,
        );

        // Attempt to store a network response with an invalid depth of
        // 1 (the peer is a VFN, but VFNs can't connect to other VFN networks).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Vfn,
            PeerRole::ValidatorFullNode,
            1,
            None,
        );

        // Attempt to store a network response with a valid depth of
        // 3 (the peer is a PFN).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Public,
            PeerRole::Unknown,
            3,
            Some(3),
        );

        // Attempt to store a network response with a valid depth of
        // 2 (the peer is a validator that is disconnected from the set).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Vfn,
            PeerRole::Validator,
            2,
            Some(2),
        );

        // Attempt to store a network response with a valid depth of
        // 0 (the peer is a validator).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Vfn,
            PeerRole::Validator,
            0,
            Some(0),
        );
    }

    #[test]
    fn test_sanity_check_distance_pfn() {
        // Create the network info state for a PFN
        let mut network_info_state = create_network_info_state(RoleType::FullNode);

        // Verify there is no latest network info response
        verify_empty_network_response(&network_info_state);

        // Attempt to store a network response with an invalid depth of
        // 0 (the peer is a PFN, not a validator).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Public,
            PeerRole::Unknown,
            0,
            None,
        );

        // Attempt to store a network response with an invalid depth of
        // 1 (the peer is a PFN, not a VFN).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Public,
            PeerRole::PreferredUpstream,
            1,
            None,
        );

        // Attempt to store a network response with a valid depth of
        // 2 (the peer is a VFN that has no validator connection).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Public,
            PeerRole::ValidatorFullNode,
            2,
            Some(2),
        );

        // Attempt to store a network response with a valid depth of
        // 1 (the peer is a VFN).
        handle_response_and_verify_distance(
            &mut network_info_state,
            NetworkId::Public,
            PeerRole::ValidatorFullNode,
            1,
            Some(1),
        );

        // Handle two valid responses from a PFN
        for distance_from_validators in [2, 3] {
            handle_response_and_verify_distance(
                &mut network_info_state,
                NetworkId::Public,
                PeerRole::Unknown,
                distance_from_validators,
                Some(distance_from_validators),
            );
        }
    }

    /// Creates a network info state using the given role type
    fn create_network_info_state(role: RoleType) -> NetworkInfoState {
        let node_config = NodeConfig {
            base: BaseConfig {
                role,
                ..Default::default()
            },
            ..Default::default()
        };
        NetworkInfoState::new(node_config, TimeService::mock())
    }

    /// Handles a monitoring service response from a peer
    /// and verifies the latest stored distance.
    fn handle_response_and_verify_distance(
        network_info_state: &mut NetworkInfoState,
        network_id: NetworkId,
        peer_role: PeerRole,
        distance_from_validators: u64,
        latest_stored_distance: Option<u64>,
    ) {
        // Handle the monitoring service response
        handle_monitoring_service_response(
            network_info_state,
            network_id,
            peer_role,
            distance_from_validators,
        );

        // Verify that the latest stored distance is correct
        match latest_stored_distance {
            Some(latest_stored_distance) => {
                verify_network_response_distance(network_info_state, latest_stored_distance);
            },
            None => {
                verify_empty_network_response(network_info_state);
            },
        }
    }

    /// Handles a monitoring service response from a peer
    fn handle_monitoring_service_response(
        network_info_state: &mut NetworkInfoState,
        network_id: NetworkId,
        peer_role: PeerRole,
        distance_from_validators: u64,
    ) {
        // Create a new peer metadata entry
        let peer_network_id = PeerNetworkId::new(network_id, PeerId::random());
        let connection_metadata = ConnectionMetadata::new(
            peer_network_id.peer_id(),
            ConnectionId::default(),
            NetworkAddress::from_str(TEST_NETWORK_ADDRESS).unwrap(),
            ConnectionOrigin::Outbound,
            MessagingProtocolVersion::V1,
            ProtocolIdSet::empty(),
            peer_role,
        );
        let peer_metadata = PeerMetadata::new(connection_metadata);

        // Create the service response
        let peer_monitoring_service_response =
            PeerMonitoringServiceResponse::NetworkInformation(NetworkInformationResponse {
                connected_peers: Default::default(),
                distance_from_validators,
            });

        // Handle the response
        network_info_state.handle_monitoring_service_response(
            &peer_network_id,
            peer_metadata,
            PeerMonitoringServiceRequest::GetNetworkInformation,
            peer_monitoring_service_response,
            0.0,
        );
    }

    /// Verifies that there is no latest network info response stored
    fn verify_empty_network_response(network_info_state: &NetworkInfoState) {
        assert!(network_info_state
            .get_latest_network_info_response()
            .is_none());
    }

    /// Verifies that the latest network info response has a valid distance
    fn verify_network_response_distance(
        network_info_state: &NetworkInfoState,
        distance_from_validators: u64,
    ) {
        let network_info_response = network_info_state
            .get_latest_network_info_response()
            .unwrap();
        assert_eq!(
            network_info_response.distance_from_validators,
            distance_from_validators
        );
    }
}
