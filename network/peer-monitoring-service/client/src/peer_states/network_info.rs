// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    peer_states::{key_value::StateValueInterface, request_tracker::RequestTracker},
    Error, LogEntry, LogEvent, LogSchema,
};
use aptos_config::{
    config::{BaseConfig, NetworkMonitoringConfig, NodeConfig, RoleType},
    network_id::PeerNetworkId,
};
use aptos_infallible::RwLock;
use aptos_logger::warn;
use aptos_network::application::metadata::PeerMetadata;
use aptos_peer_monitoring_service_types::{
    NetworkInformationResponse, PeerMonitoringServiceRequest, PeerMonitoringServiceResponse,
    MAX_DISTANCE_FROM_VALIDATORS,
};
use aptos_time_service::TimeService;
use std::sync::Arc;

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
        mut network_info_response: NetworkInformationResponse,
        peer_network_id: &PeerNetworkId,
        peer_metadata: PeerMetadata,
    ) {
        // Update the request tracker with a successful response
        self.request_tracker.write().record_response_success();

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
                    RoleType::FullNode => network_id.is_public_network(), // We're a PFN
                };
                peer_is_vfn && peer_has_correct_network
            },
            distance_from_validators => {
                // The depth must be less than or equal to the max
                distance_from_validators <= MAX_DISTANCE_FROM_VALIDATORS
            },
        };

        // If the depth did not pass our sanity checks, store the max
        if !is_valid_depth {
            warn!(LogSchema::new(LogEntry::NetworkInfoRequest)
                .event(LogEvent::InvalidResponse)
                .peer(peer_network_id)
                .message(&format!(
                    "Peer returned invalid depth from validators: {}",
                    network_info_response.distance_from_validators
                )));
            network_info_response.distance_from_validators = MAX_DISTANCE_FROM_VALIDATORS;
        }

        // Save the network info
        self.recorded_network_info_response = Some(network_info_response);
    }

    /// Handles a request failure for the specified peer
    fn handle_request_failure(&self) {
        // Update the number of ping failures for the request tracker
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

        // Store the new latency ping result
        self.record_network_info_response(network_info_response, peer_network_id, peer_metadata);
    }

    fn handle_monitoring_service_response_error(
        &self,
        peer_network_id: &PeerNetworkId,
        error: Error,
    ) {
        // Record the failure
        self.request_tracker.write().record_response_failure();

        // Log the error
        warn!(LogSchema::new(LogEntry::NetworkInfoRequest)
            .event(LogEvent::ResponseError)
            .message("Error encountered when requesting network information from the peer!")
            .peer(peer_network_id)
            .error(&error));
    }
}
