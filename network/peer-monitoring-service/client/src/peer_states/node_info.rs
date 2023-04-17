// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    peer_states::{key_value::StateValueInterface, request_tracker::RequestTracker},
    Error, LogEntry, LogEvent, LogSchema,
};
use aptos_config::{config::NodeMonitoringConfig, network_id::PeerNetworkId};
use aptos_infallible::RwLock;
use aptos_logger::warn;
use aptos_network::application::metadata::PeerMetadata;
use aptos_peer_monitoring_service_types::{
    request::PeerMonitoringServiceRequest,
    response::{NodeInformationResponse, PeerMonitoringServiceResponse},
};
use aptos_time_service::TimeService;
use std::{collections::BTreeMap, sync::Arc};

// The maximum number of entries and string lengths in the build info map
const MAX_BUILD_INFORMATION_ENTRIES: usize = 100;
const MAX_BUILD_INFORMATION_STRING_LENGTH: usize = 200;

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
        let mut node_info_response = match monitoring_service_response {
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

        // Verify the build information does not have too many entries
        let num_build_information_entries = node_info_response.build_information.len();
        if num_build_information_entries > MAX_BUILD_INFORMATION_ENTRIES {
            warn!(LogSchema::new(LogEntry::NodeInfoRequest)
                .event(LogEvent::ResponseError)
                .peer(peer_network_id)
                .message(&format!(
                    "The build information is too large! Got length {:?} but the max is {:?}!",
                    num_build_information_entries, MAX_BUILD_INFORMATION_ENTRIES
                )));
            self.handle_request_failure();
            return;
        }

        // Trim the entries of the build information (if they are too long)
        let build_information = node_info_response
            .build_information
            .iter()
            .map(|(key, value)| (trim_to_max_length(key), trim_to_max_length(value)));
        node_info_response.build_information = BTreeMap::from_iter(build_information);

        // Store the new latency ping result
        self.record_node_info_response(node_info_response);
    }

    fn handle_monitoring_service_response_error(
        &self,
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
}

/// Trims the specified string to the maximum length
/// allowed in the build info map and returns the
/// (possibly) truncated string.
fn trim_to_max_length(string_to_trim: &str) -> String {
    if string_to_trim.len() > MAX_BUILD_INFORMATION_STRING_LENGTH {
        string_to_trim[0..MAX_BUILD_INFORMATION_STRING_LENGTH].into()
    } else {
        string_to_trim.into()
    }
}

#[cfg(test)]
mod test {
    use crate::peer_states::{
        key_value::StateValueInterface,
        node_info::{
            NodeInfoState, MAX_BUILD_INFORMATION_ENTRIES, MAX_BUILD_INFORMATION_STRING_LENGTH,
        },
    };
    use aptos_config::{
        config::{NodeMonitoringConfig, PeerRole},
        network_id::PeerNetworkId,
    };
    use aptos_netcore::transport::ConnectionOrigin;
    use aptos_network::{
        application::metadata::PeerMetadata,
        protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolIdSet},
        transport::{ConnectionId, ConnectionMetadata},
    };
    use aptos_peer_monitoring_service_types::{
        request::PeerMonitoringServiceRequest,
        response::{NodeInformationResponse, PeerMonitoringServiceResponse},
    };
    use aptos_time_service::TimeService;
    use aptos_types::network_address::NetworkAddress;
    use std::{collections::BTreeMap, str::FromStr, time::Duration};

    // Useful test constants
    const TEST_NETWORK_ADDRESS: &str = "/ip4/127.0.0.1/tcp/8081";

    #[test]
    fn test_sanity_check_build_info() {
        // Create the node info state
        let node_monitoring_config = NodeMonitoringConfig::default();
        let time_service = TimeService::mock();
        let mut node_info_state = NodeInfoState::new(node_monitoring_config, time_service);

        // Verify the initial node info state
        verify_empty_node_response(&node_info_state);

        // Generate the test data
        let highest_synced_epoch = 0;
        let highest_synced_version = 100;
        let ledger_timestamp_usecs = 200;
        let lowest_available_version = highest_synced_version - 10;
        let uptime = Duration::from_millis(999);

        // Create a build info map that has too many entries (with long strings)
        let mut build_information = BTreeMap::new();
        for i in 0..MAX_BUILD_INFORMATION_ENTRIES + 1 {
            let counter_string = format!("{}", i);
            let key = repeat_substring(&counter_string, MAX_BUILD_INFORMATION_STRING_LENGTH + 1);
            let value = repeat_substring(&counter_string, MAX_BUILD_INFORMATION_STRING_LENGTH + 1);
            build_information.insert(key, value);
        }

        // Create the service response
        let node_information_response = NodeInformationResponse {
            build_information: build_information.clone(),
            highest_synced_epoch,
            highest_synced_version,
            ledger_timestamp_usecs,
            lowest_available_version,
            uptime,
        };

        // Handle the node info response
        handle_monitoring_service_response(&mut node_info_state, node_information_response);

        // Verify there is still no latest node info response
        verify_empty_node_response(&node_info_state);

        // Verify the number of consecutive request failures has increased
        let num_consecutive_failures = node_info_state
            .get_request_tracker()
            .read()
            .get_num_consecutive_failures();
        assert_eq!(num_consecutive_failures, 1);

        // Modify the build info map to be the correct size
        build_information.pop_last().unwrap();

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
        handle_monitoring_service_response(&mut node_info_state, node_information_response);

        // Verify the number of consecutive request failures has reset
        let num_consecutive_failures = node_info_state
            .get_request_tracker()
            .read()
            .get_num_consecutive_failures();
        assert_eq!(num_consecutive_failures, 0);

        // Verify the basic properties of the latest node info response
        let latest_node_info_response = node_info_state.get_latest_node_info_response().unwrap();
        assert_eq!(
            latest_node_info_response.highest_synced_epoch,
            highest_synced_epoch
        );
        assert_eq!(
            latest_node_info_response.highest_synced_version,
            highest_synced_version
        );
        assert_eq!(
            latest_node_info_response.ledger_timestamp_usecs,
            ledger_timestamp_usecs
        );
        assert_eq!(
            latest_node_info_response.lowest_available_version,
            lowest_available_version
        );
        assert_eq!(latest_node_info_response.uptime, uptime);

        // Verify the latest node info response contains trimmed build info strings
        for (key, value) in latest_node_info_response.build_information.iter() {
            assert!(key.len() <= MAX_BUILD_INFORMATION_STRING_LENGTH);
            assert!(value.len() <= MAX_BUILD_INFORMATION_STRING_LENGTH);
        }
    }

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
            let highest_synced_epoch = i;
            let highest_synced_version = (i + 1) * 100;
            let ledger_timestamp_usecs = (i + 1) * 200;
            let lowest_available_version = highest_synced_version - 10;
            let uptime = Duration::from_millis(i * 999);

            // Generate the build info map
            let mut build_information = BTreeMap::new();
            build_information.insert(i.to_string(), i.to_string());

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

    /// Repeats the given substring the specified number of times
    /// and returns the resulting string.
    fn repeat_substring(substring: &str, num_repeats: usize) -> String {
        let mut result = String::new();
        for _ in 0..num_repeats {
            result.push_str(substring);
        }
        result
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
