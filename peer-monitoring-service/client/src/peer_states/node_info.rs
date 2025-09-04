// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics,
    peer_states::{key_value::StateValueInterface, request_tracker::RequestTracker},
    Error, LogEntry, LogEvent, LogSchema,
};
use velor_config::{config::NodeMonitoringConfig, network_id::PeerNetworkId};
use velor_infallible::RwLock;
use velor_logger::warn;
use velor_network::application::metadata::PeerMetadata;
use velor_peer_monitoring_service_types::{
    request::PeerMonitoringServiceRequest,
    response::{NodeInformationResponse, PeerMonitoringServiceResponse},
};
use velor_time_service::TimeService;
use std::{
    fmt,
    fmt::{Display, Formatter},
    sync::Arc,
};

/// A simple container that holds a single peer's node info
#[derive(Clone, Debug)]
pub struct NodeInfoState {
    node_monitoring_config: NodeMonitoringConfig, // The config for node monitoring
    recorded_node_info_response: Option<NodeInformationResponse>, // The last node info response
    request_tracker: Arc<RwLock<RequestTracker>>, // The request tracker for node info requests
}

impl NodeInfoState {
    pub fn new(node_monitoring_config: NodeMonitoringConfig, time_service: TimeService) -> Self {
        let request_tracker = RequestTracker::new(
            node_monitoring_config.node_info_request_interval_ms,
            time_service,
        );

        Self {
            node_monitoring_config,
            recorded_node_info_response: None,
            request_tracker: Arc::new(RwLock::new(request_tracker)),
        }
    }

    /// Records the new node info response for the peer
    pub fn record_node_info_response(&mut self, node_info_response: NodeInformationResponse) {
        // Update the request tracker with a successful response
        self.request_tracker.write().record_response_success();

        // Save the node info
        self.recorded_node_info_response = Some(node_info_response);
    }

    /// Handles a request failure for the specified peer
    fn handle_request_failure(&self) {
        self.request_tracker.write().record_response_failure();
    }

    /// Returns the latest node info response
    pub fn get_latest_node_info_response(&self) -> Option<NodeInformationResponse> {
        self.recorded_node_info_response.clone()
    }
}

impl StateValueInterface for NodeInfoState {
    fn create_monitoring_service_request(&mut self) -> PeerMonitoringServiceRequest {
        PeerMonitoringServiceRequest::GetNodeInformation
    }

    fn get_request_timeout_ms(&self) -> u64 {
        self.node_monitoring_config.node_info_request_timeout_ms
    }

    fn get_request_tracker(&self) -> Arc<RwLock<RequestTracker>> {
        self.request_tracker.clone()
    }

    fn handle_monitoring_service_response(
        &mut self,
        peer_network_id: &PeerNetworkId,
        _peer_metadata: PeerMetadata,
        _monitoring_service_request: PeerMonitoringServiceRequest,
        monitoring_service_response: PeerMonitoringServiceResponse,
        _response_time_secs: f64,
    ) {
        // Verify the response type is valid
        let node_info_response = match monitoring_service_response {
            PeerMonitoringServiceResponse::NodeInformation(node_information_response) => {
                node_information_response
            },
            _ => {
                warn!(LogSchema::new(LogEntry::NodeInfoRequest)
                    .event(LogEvent::ResponseError)
                    .peer(peer_network_id)
                    .message(
                        "An unexpected response was received instead of a node info response!"
                    ));
                self.handle_request_failure();
                return;
            },
        };

        // Store the new latency ping result
        self.record_node_info_response(node_info_response);
    }

    fn handle_monitoring_service_response_error(
        &mut self,
        peer_network_id: &PeerNetworkId,
        error: Error,
    ) {
        // Handle the failure
        self.handle_request_failure();

        // Log the error
        warn!(LogSchema::new(LogEntry::NodeInfoRequest)
            .event(LogEvent::ResponseError)
            .message("Error encountered when requesting node information from the peer!")
            .peer(peer_network_id)
            .error(&error));
    }

    fn update_peer_state_metrics(&self, peer_network_id: &PeerNetworkId) {
        if let Some(node_info_response) = self.get_latest_node_info_response() {
            // Update the uptime metric
            let node_uptime = node_info_response.uptime;
            let uptime_in_hours = node_uptime.as_secs_f64() / 3600.0; // Convert to hours
            metrics::observe_value(&metrics::NODE_UPTIME, peer_network_id, uptime_in_hours);
        }
    }
}

impl Display for NodeInfoState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NodeInfoState {{ recorded_node_info_response: {:?} }}",
            self.recorded_node_info_response
        )
    }
}

#[cfg(test)]
mod test {
    use crate::peer_states::{key_value::StateValueInterface, node_info::NodeInfoState};
    use velor_config::{
        config::{NodeMonitoringConfig, PeerRole},
        network_id::PeerNetworkId,
    };
    use velor_netcore::transport::ConnectionOrigin;
    use velor_network::{
        application::metadata::PeerMetadata,
        protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolIdSet},
        transport::{ConnectionId, ConnectionMetadata},
    };
    use velor_peer_monitoring_service_types::{
        request::PeerMonitoringServiceRequest,
        response::{NodeInformationResponse, PeerMonitoringServiceResponse},
    };
    use velor_time_service::TimeService;
    use velor_types::network_address::NetworkAddress;
    use std::{str::FromStr, time::Duration};

    // Useful test constants
    const TEST_NETWORK_ADDRESS: &str = "/ip4/127.0.0.1/tcp/8081";

    #[test]
    fn test_verify_node_info_state() {
        // Create the node info state
        let node_monitoring_config = NodeMonitoringConfig::default();
        let time_service = TimeService::mock();
        let mut node_info_state = NodeInfoState::new(node_monitoring_config, time_service);

        // Verify the initial node info state
        verify_empty_node_response(&node_info_state);

        // Handle several valid node info responses and verify the state
        for i in 0..10 {
            // Generate the test data
            let build_information = velor_build_info::get_build_information();
            let highest_synced_epoch = i;
            let highest_synced_version = (i + 1) * 100;
            let ledger_timestamp_usecs = (i + 1) * 200;
            let lowest_available_version = highest_synced_version - 10;
            let uptime = Duration::from_millis(i * 999);

            // Create the service response
            let node_information_response = NodeInformationResponse {
                build_information,
                highest_synced_epoch,
                highest_synced_version,
                ledger_timestamp_usecs,
                lowest_available_version,
                uptime,
            };

            // Handle the node info response
            handle_monitoring_service_response(
                &mut node_info_state,
                node_information_response.clone(),
            );

            // Verify the latest node info state
            verify_node_info_state(&node_info_state, node_information_response);
        }
    }

    /// Handles a monitoring service response from a peer
    fn handle_monitoring_service_response(
        node_info_state: &mut NodeInfoState,
        node_information_response: NodeInformationResponse,
    ) {
        // Create a new peer metadata entry
        let peer_network_id = PeerNetworkId::random();
        let connection_metadata = ConnectionMetadata::new(
            peer_network_id.peer_id(),
            ConnectionId::default(),
            NetworkAddress::from_str(TEST_NETWORK_ADDRESS).unwrap(),
            ConnectionOrigin::Outbound,
            MessagingProtocolVersion::V1,
            ProtocolIdSet::empty(),
            PeerRole::Validator,
        );
        let peer_metadata = PeerMetadata::new(connection_metadata);

        // Create the service response
        let peer_monitoring_service_response =
            PeerMonitoringServiceResponse::NodeInformation(node_information_response);

        // Handle the response
        node_info_state.handle_monitoring_service_response(
            &peer_network_id,
            peer_metadata,
            PeerMonitoringServiceRequest::GetNodeInformation,
            peer_monitoring_service_response,
            0.0,
        );
    }

    /// Verifies that there is no latest node info response stored
    fn verify_empty_node_response(node_info_state: &NodeInfoState) {
        assert!(node_info_state.get_latest_node_info_response().is_none());
    }

    /// Verifies that the latest node info response is valid
    fn verify_node_info_state(
        node_info_state: &NodeInfoState,
        expected_node_info_response: NodeInformationResponse,
    ) {
        let latest_node_info_response = node_info_state.get_latest_node_info_response().unwrap();
        assert_eq!(latest_node_info_response, expected_node_info_response);
    }
}
