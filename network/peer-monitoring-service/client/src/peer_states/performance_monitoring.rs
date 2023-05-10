// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    peer_states::{key_value::StateValueInterface, request_tracker::RequestTracker},
    Error, LogEntry, LogEvent, LogSchema,
};
use aptos_config::{config::PerformanceMonitoringConfig, network_id::PeerNetworkId};
use aptos_infallible::RwLock;
use aptos_logger::{error, warn};
use aptos_network::application::metadata::PeerMetadata;
use aptos_peer_monitoring_service_types::{
    request::{PeerMonitoringServiceRequest, PerformanceMonitoringRequest},
    response::{PeerMonitoringServiceResponse, PerformanceMonitoringResponse},
};
use aptos_time_service::TimeService;
use std::sync::Arc;

/// A simple container that holds a single peer's performance monitoring state
#[derive(Clone, Debug)]
pub struct PerformanceMonitoringState {
    performance_monitoring_config: PerformanceMonitoringConfig, // The config for performance monitoring
    recorded_performance_response: Option<PerformanceMonitoringResponse>, // The last performance response
    request_counter: u64, // The monotonically increasing counter for each request
    request_data: Vec<u8>, // The data to send in each request
    request_tracker: Arc<RwLock<RequestTracker>>, // The request tracker for performance monitoring
}

impl PerformanceMonitoringState {
    pub fn new(
        performance_monitoring_config: PerformanceMonitoringConfig,
        time_service: TimeService,
    ) -> Self {
        // Create the request tracker
        let request_interval_usecs = performance_monitoring_config.rpc_interval_usec;
        let request_tracker =
            RequestTracker::new_with_microseconds(request_interval_usecs, time_service);

        Self {
            performance_monitoring_config,
            recorded_performance_response: None,
            request_counter: 0,
            request_data: vec![],
            request_tracker: Arc::new(RwLock::new(request_tracker)),
        }
    }

    /// Returns the current request counter and increments it internally
    pub fn get_and_increment_request_counter(&mut self) -> u64 {
        let request_counter = self.request_counter;
        self.request_counter += 1;
        request_counter
    }

    /// Returns the latest performance response
    pub fn get_latest_performance_monitoring_response(
        &self,
    ) -> Option<PerformanceMonitoringResponse> {
        self.recorded_performance_response.clone()
    }

    /// Records the new performance response for the peer
    pub fn record_performance_monitoring_response(
        &mut self,
        performance_response: PerformanceMonitoringResponse,
    ) {
        // Update the request tracker with a successful response
        self.request_tracker.write().record_response_success();

        // Save the response
        self.recorded_performance_response = Some(performance_response);
    }

    /// Handles a request failure for the specified peer
    fn handle_request_failure(&self) {
        self.request_tracker.write().record_response_failure();
    }

    /// Returns the latest performance response
    pub fn get_latest_performance_response(&self) -> Option<PerformanceMonitoringResponse> {
        self.recorded_performance_response.clone()
    }
}

impl StateValueInterface for PerformanceMonitoringState {
    fn create_monitoring_service_request(&mut self) -> PeerMonitoringServiceRequest {
        // Create the request data (if it hasn't already been created yet)
        if self.request_data.is_empty() {
            self.request_data = create_request_data(&self.performance_monitoring_config);
        }

        // Return the request
        PeerMonitoringServiceRequest::PerformanceMonitoringRequest(PerformanceMonitoringRequest {
            request_counter: self.get_and_increment_request_counter(),
            data: self.request_data.clone(),
        })
    }

    fn get_request_timeout_ms(&self) -> u64 {
        self.performance_monitoring_config.rpc_timeout_ms
    }

    fn get_request_tracker(&self) -> Arc<RwLock<RequestTracker>> {
        self.request_tracker.clone()
    }

    fn handle_monitoring_service_response(
        &mut self,
        peer_network_id: &PeerNetworkId,
        _peer_metadata: PeerMetadata,
        monitoring_service_request: PeerMonitoringServiceRequest,
        monitoring_service_response: PeerMonitoringServiceResponse,
        _response_time_secs: f64,
    ) {
        // Verify the request type is correctly formed
        let monitoring_service_request = match monitoring_service_request {
            PeerMonitoringServiceRequest::PerformanceMonitoringRequest(
                monitoring_service_request,
            ) => monitoring_service_request,
            request => {
                error!(LogSchema::new(LogEntry::SendRequest)
                    .event(LogEvent::UnexpectedErrorEncountered)
                    .peer(peer_network_id)
                    .request(&request)
                    .message("An unexpected request was sent instead of a performance monitoring request!"));
                self.handle_request_failure();
                return;
            },
        };

        // Verify the response type is valid
        let performance_monitoring_response = match monitoring_service_response {
            PeerMonitoringServiceResponse::PerformanceMonitoring(
                performance_monitoring_response,
            ) => performance_monitoring_response,
            _ => {
                warn!(LogSchema::new(LogEntry::PerformanceMonitoringRequest)
                    .event(LogEvent::ResponseError)
                    .peer(peer_network_id)
                    .message("An unexpected response was received instead of a performance monitoring response!"));
                self.handle_request_failure();
                return;
            },
        };

        // Verify the request counter is correct
        let request_counter = monitoring_service_request.request_counter;
        let response_counter = performance_monitoring_response.response_counter;
        if request_counter != response_counter {
            warn!(LogSchema::new(LogEntry::PerformanceMonitoringRequest)
                .event(LogEvent::ResponseError)
                .peer(peer_network_id)
                .message(&format!(
                    "Peer responded with the incorrect request counter! Expected: {:?}, found: {:?}",
                    request_counter, response_counter
                )));
            self.handle_request_failure();
            return;
        }

        // Store the new performance response
        self.record_performance_monitoring_response(performance_monitoring_response);
    }

    fn handle_monitoring_service_response_error(
        &self,
        peer_network_id: &PeerNetworkId,
        error: Error,
    ) {
        // Handle the failure
        self.handle_request_failure();

        // Log the error
        warn!(LogSchema::new(LogEntry::PerformanceMonitoringRequest)
            .event(LogEvent::ResponseError)
            .message("Error encountered when sending a performance request to the peer!")
            .peer(peer_network_id)
            .error(&error));
    }
}

/// Creates the request data for the performance monitoring requests
fn create_request_data(performance_monitoring_config: &PerformanceMonitoringConfig) -> Vec<u8> {
    // Calculate the data size
    let data_size = if performance_monitoring_config.enable_direct_send_testing {
        performance_monitoring_config.direct_send_data_size
    } else {
        performance_monitoring_config.rpc_data_size
    };

    // Generate the random request data
    (0..data_size).map(|_| rand::random::<u8>()).collect()
}

#[cfg(test)]
mod test {
    use crate::peer_states::{
        key_value::StateValueInterface, performance_monitoring::PerformanceMonitoringState,
    };
    use aptos_config::{
        config::{PeerRole, PerformanceMonitoringConfig},
        network_id::{NetworkId, PeerNetworkId},
    };
    use aptos_netcore::transport::ConnectionOrigin;
    use aptos_network::{
        application::metadata::PeerMetadata,
        protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolIdSet},
        transport::{ConnectionId, ConnectionMetadata},
    };
    use aptos_peer_monitoring_service_types::{
        request::{PeerMonitoringServiceRequest, PerformanceMonitoringRequest},
        response::{PeerMonitoringServiceResponse, PerformanceMonitoringResponse},
    };
    use aptos_time_service::TimeService;
    use aptos_types::{network_address::NetworkAddress, PeerId};
    use std::str::FromStr;

    // Useful test constants
    const TEST_NETWORK_ADDRESS: &str = "/ip4/127.0.0.1/tcp/8081";

    #[test]
    fn test_verify_performance_monitoring_state() {
        // Create the performance monitoring state
        let performance_monitoring_config = PerformanceMonitoringConfig::default();
        let time_service = TimeService::mock();
        let mut performance_monitoring_state =
            PerformanceMonitoringState::new(performance_monitoring_config, time_service);

        // Verify the initial performance monitoring state
        verify_empty_performance_monitoring_response(&performance_monitoring_state);
        assert_eq!(performance_monitoring_state.request_counter, 0);
        assert!(performance_monitoring_state.request_data.is_empty());

        // Attempt to handle an invalid monitoring response with mismatched request counters
        let request_counter = performance_monitoring_state.get_and_increment_request_counter();
        handle_monitoring_service_response(
            &mut performance_monitoring_state,
            request_counter,
            request_counter + 1,
        );

        // Verify there is still no recorded response
        verify_empty_performance_monitoring_response(&performance_monitoring_state);

        // Handle several valid monitoring responses
        let num_responses = 10;
        for _ in 0..num_responses {
            // Handle the monitoring response
            let request_counter = performance_monitoring_state.get_and_increment_request_counter();
            handle_monitoring_service_response(
                &mut performance_monitoring_state,
                request_counter,
                request_counter,
            );
        }

        // Verify the performance monitoring state
        verify_performance_monitoring_state(&performance_monitoring_state, num_responses + 1);
    }

    /// Handles a monitoring service response from a peer
    fn handle_monitoring_service_response(
        performance_monitoring_state: &mut PerformanceMonitoringState,
        request_counter: u64,
        response_counter: u64,
    ) {
        // Create a new peer metadata entry
        let peer_network_id = PeerNetworkId::new(NetworkId::Validator, PeerId::random());
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

        // Create the service request
        let peer_monitoring_service_request =
            PeerMonitoringServiceRequest::PerformanceMonitoringRequest(
                PerformanceMonitoringRequest {
                    request_counter,
                    data: vec![],
                },
            );

        // Create the service response
        let peer_monitoring_service_response =
            PeerMonitoringServiceResponse::PerformanceMonitoring(PerformanceMonitoringResponse {
                response_counter,
            });

        // Handle the response
        performance_monitoring_state.handle_monitoring_service_response(
            &peer_network_id,
            peer_metadata,
            peer_monitoring_service_request,
            peer_monitoring_service_response,
            0.0,
        );
    }

    /// Verifies that there is no recorded performance monitoring response
    fn verify_empty_performance_monitoring_response(
        performance_monitoring_state: &PerformanceMonitoringState,
    ) {
        assert!(performance_monitoring_state
            .recorded_performance_response
            .is_none());
    }

    /// Verifies that the latest performance monitoring response is valid
    fn verify_performance_monitoring_state(
        performance_monitoring_state: &PerformanceMonitoringState,
        expected_request_counter: u64,
    ) {
        // Verify the request counter matches the expected value
        assert_eq!(
            performance_monitoring_state.request_counter,
            expected_request_counter
        );

        // Verify the latest performance monitoring response
        let performance_monitoring_response = performance_monitoring_state
            .get_latest_performance_monitoring_response()
            .unwrap();
        assert_eq!(
            performance_monitoring_response.response_counter,
            expected_request_counter - 1
        );
    }
}
