// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics,
    peer_states::{key_value::StateValueInterface, request_tracker::RequestTracker},
    Error, LogEntry, LogEvent, LogSchema,
};
use aptos_config::{config::LatencyMonitoringConfig, network_id::PeerNetworkId};
use aptos_infallible::RwLock;
use aptos_logger::{error, warn};
use aptos_network::application::metadata::PeerMetadata;
use aptos_peer_monitoring_service_types::{
    request::{LatencyPingRequest, PeerMonitoringServiceRequest},
    response::PeerMonitoringServiceResponse,
};
use aptos_time_service::TimeService;
use std::{
    collections::BTreeMap,
    fmt,
    fmt::{Display, Formatter},
    sync::Arc,
};

/// A simple container that holds a peer's latency info
#[derive(Clone, Debug)]
pub struct LatencyInfoState {
    latency_monitoring_config: LatencyMonitoringConfig, // The config for latency monitoring
    latency_ping_counter: u64, // The monotonically increasing counter for each ping
    recorded_latency_ping_durations_secs: BTreeMap<u64, f64>, // Successful ping durations by counter (secs)
    request_tracker: Arc<RwLock<RequestTracker>>, // The request tracker for latency ping requests
}

impl LatencyInfoState {
    pub fn new(
        latency_monitoring_config: LatencyMonitoringConfig,
        time_service: TimeService,
    ) -> Self {
        let request_tracker = RequestTracker::new(
            latency_monitoring_config.latency_ping_interval_ms,
            time_service,
        );

        Self {
            latency_monitoring_config,
            latency_ping_counter: 0,
            recorded_latency_ping_durations_secs: BTreeMap::new(),
            request_tracker: Arc::new(RwLock::new(request_tracker)),
        }
    }

    /// Returns the current latency ping counter and increments it internally
    pub fn get_and_increment_latency_ping_counter(&mut self) -> u64 {
        let latency_ping_counter = self.latency_ping_counter;
        self.latency_ping_counter += 1;
        latency_ping_counter
    }

    /// Handles a ping failure for the specified peer
    fn handle_request_failure(&self, peer_network_id: &PeerNetworkId) {
        // Update the number of ping failures for the request tracker
        self.request_tracker.write().record_response_failure();

        // TODO: If the number of ping failures is too high, disconnect from the node
        let num_consecutive_failures = self.request_tracker.read().get_num_consecutive_failures();
        if num_consecutive_failures >= self.latency_monitoring_config.max_latency_ping_failures {
            warn!(LogSchema::new(LogEntry::LatencyPing)
                .event(LogEvent::TooManyPingFailures)
                .peer(peer_network_id)
                .message("Too many ping failures occurred for the peer!"));
        }
    }

    /// Records the new latency ping entry for the peer and resets the
    /// consecutive failure counter.
    pub fn record_new_latency_and_reset_failures(
        &mut self,
        latency_ping_counter: u64,
        latency_ping_time_secs: f64,
    ) {
        // Update the request tracker with a successful response
        self.request_tracker.write().record_response_success();

        // Save the latency ping time
        self.recorded_latency_ping_durations_secs
            .insert(latency_ping_counter, latency_ping_time_secs);

        // Perform garbage collection on the recorded latency pings
        let max_num_latency_pings_to_retain = self
            .latency_monitoring_config
            .max_num_latency_pings_to_retain;
        if self.recorded_latency_ping_durations_secs.len() > max_num_latency_pings_to_retain {
            // We only need to pop a single element because insertion only happens in this method.
            // Thus, the size can only ever grow to be 1 greater than the max.
            let _ = self.recorded_latency_ping_durations_secs.pop_first();
        }
    }

    /// Returns the average latency ping in seconds. If no latency
    /// pings have been recorded, None is returned.
    pub fn get_average_latency_ping_secs(&self) -> Option<f64> {
        let num_latency_pings = self.recorded_latency_ping_durations_secs.len();
        if num_latency_pings > 0 {
            let average_latency_secs_sum: f64 =
                self.recorded_latency_ping_durations_secs.values().sum();
            Some(average_latency_secs_sum / num_latency_pings as f64)
        } else {
            None
        }
    }

    /// Returns the latest latency ping in seconds. If no latency
    /// pings have been recorded, None is returned.
    pub fn get_latest_latency_ping_secs(&self) -> Option<f64> {
        self.recorded_latency_ping_durations_secs
            .last_key_value()
            .map(|(_, value)| *value)
    }

    /// Returns a copy of the recorded latency pings for test purposes
    #[cfg(test)]
    pub fn get_recorded_latency_pings(&self) -> BTreeMap<u64, f64> {
        self.recorded_latency_ping_durations_secs.clone()
    }
}

impl StateValueInterface for LatencyInfoState {
    fn create_monitoring_service_request(&mut self) -> PeerMonitoringServiceRequest {
        let ping_counter = self.get_and_increment_latency_ping_counter();
        PeerMonitoringServiceRequest::LatencyPing(LatencyPingRequest { ping_counter })
    }

    fn get_request_timeout_ms(&self) -> u64 {
        self.latency_monitoring_config.latency_ping_timeout_ms
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
        response_time_secs: f64,
    ) {
        // Verify the request type is correctly formed
        let latency_ping_request = match monitoring_service_request {
            PeerMonitoringServiceRequest::LatencyPing(latency_ping_request) => latency_ping_request,
            request => {
                error!(LogSchema::new(LogEntry::LatencyPing)
                    .event(LogEvent::UnexpectedErrorEncountered)
                    .peer(peer_network_id)
                    .request(&request)
                    .message("An unexpected request was sent instead of a latency ping!"));
                self.handle_request_failure(peer_network_id);
                return;
            },
        };

        // Verify the response type is valid
        let latency_ping_response = match monitoring_service_response {
            PeerMonitoringServiceResponse::LatencyPing(latency_ping_response) => {
                latency_ping_response
            },
            _ => {
                warn!(LogSchema::new(LogEntry::LatencyPing)
                    .event(LogEvent::ResponseError)
                    .peer(peer_network_id)
                    .message("An unexpected response was received instead of a latency ping!"));
                self.handle_request_failure(peer_network_id);
                return;
            },
        };

        // Verify the latency ping response contains the correct counter
        let request_ping_counter = latency_ping_request.ping_counter;
        let response_ping_counter = latency_ping_response.ping_counter;
        if request_ping_counter != response_ping_counter {
            warn!(LogSchema::new(LogEntry::LatencyPing)
                .event(LogEvent::PeerPingError)
                .peer(peer_network_id)
                .message(&format!(
                    "Peer responded with the incorrect ping counter! Expected: {:?}, found: {:?}",
                    request_ping_counter, response_ping_counter
                )));
            self.handle_request_failure(peer_network_id);
            return;
        }

        // Store the new latency ping result
        self.record_new_latency_and_reset_failures(request_ping_counter, response_time_secs);
    }

    fn handle_monitoring_service_response_error(
        &mut self,
        peer_network_id: &PeerNetworkId,
        error: Error,
    ) {
        // Handle the failure
        self.handle_request_failure(peer_network_id);

        // Log the error
        warn!(LogSchema::new(LogEntry::LatencyPing)
            .event(LogEvent::ResponseError)
            .message("Error encountered when pinging peer!")
            .peer(peer_network_id)
            .error(&error));
    }

    fn update_peer_state_metrics(&self, peer_network_id: &PeerNetworkId) {
        if let Some(average_latency_ping_secs) = self.get_average_latency_ping_secs() {
            // Update the average ping latency metric
            metrics::observe_value(
                &metrics::AVERAGE_PING_LATENCIES,
                peer_network_id,
                average_latency_ping_secs,
            );
        }
    }
}

impl Display for LatencyInfoState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LatencyInfoState {{ latency_ping_counter, {:?}, recorded_latency_ping_durations_secs: {:?} }}",
            self.latency_ping_counter, self.recorded_latency_ping_durations_secs,
        )
    }
}

#[cfg(test)]
mod test {
    use crate::peer_states::{key_value::StateValueInterface, latency_info::LatencyInfoState};
    use aptos_config::{
        config::{LatencyMonitoringConfig, PeerRole},
        network_id::{NetworkId, PeerNetworkId},
    };
    use aptos_netcore::transport::ConnectionOrigin;
    use aptos_network::{
        application::metadata::PeerMetadata,
        protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolIdSet},
        transport::{ConnectionId, ConnectionMetadata},
    };
    use aptos_peer_monitoring_service_types::{
        request::{LatencyPingRequest, PeerMonitoringServiceRequest},
        response::{LatencyPingResponse, PeerMonitoringServiceResponse},
    };
    use aptos_time_service::TimeService;
    use aptos_types::{network_address::NetworkAddress, PeerId};
    use rand::{rngs::OsRng, Rng};
    use std::{cmp::min, str::FromStr};

    // Useful test constants
    const TEST_NETWORK_ADDRESS: &str = "/ip4/127.0.0.1/tcp/8081";

    #[test]
    fn test_verify_latency_info_state() {
        // Create the latency info state
        let latency_monitoring_config = LatencyMonitoringConfig::default();
        let time_service = TimeService::mock();
        let mut latency_info_state = LatencyInfoState::new(latency_monitoring_config, time_service);

        // Verify the initial latency info state
        assert_eq!(latency_info_state.latency_ping_counter, 0);
        assert!(latency_info_state.get_average_latency_ping_secs().is_none());
        verify_no_recorded_pings(&mut latency_info_state);

        // Attempt to handle an invalid ping response with mismatched ping counters
        let ping_request_counter = latency_info_state.get_and_increment_latency_ping_counter();
        handle_monitoring_service_response(
            &mut latency_info_state,
            ping_request_counter,
            ping_request_counter + 1,
            0.0,
        );

        // Verify there are still no recorded latency pings
        verify_no_recorded_pings(&mut latency_info_state);

        // Handle several valid ping responses
        let num_latency_pings = latency_monitoring_config.max_num_latency_pings_to_retain;
        for _ in 0..num_latency_pings {
            // Handle the ping response
            let ping_request_counter = latency_info_state.get_and_increment_latency_ping_counter();
            let latency_ping_duration = get_random_u64() as f64;
            handle_monitoring_service_response(
                &mut latency_info_state,
                ping_request_counter,
                ping_request_counter,
                latency_ping_duration,
            );
        }

        // Verify the average latency
        let recorded_latency_pings = latency_info_state.get_recorded_latency_pings();
        assert_eq!(
            latency_info_state.get_average_latency_ping_secs().unwrap(),
            recorded_latency_pings.values().sum::<f64>() / recorded_latency_pings.len() as f64,
        );
    }

    #[test]
    fn test_verify_latency_info_garbage_collection() {
        // Create the latency info state
        let latency_monitoring_config = LatencyMonitoringConfig::default();
        let time_service = TimeService::mock();
        let mut latency_info_state = LatencyInfoState::new(latency_monitoring_config, time_service);

        // Verify the initial latency info state
        assert_eq!(latency_info_state.latency_ping_counter, 0);
        assert!(latency_info_state.get_average_latency_ping_secs().is_none());
        verify_no_recorded_pings(&mut latency_info_state);

        // Handle several valid ping responses and verify the number of stored entries
        let max_num_latency_pings_to_retain =
            latency_monitoring_config.max_num_latency_pings_to_retain as u64;
        let num_latency_pings = max_num_latency_pings_to_retain * 10;
        for i in 0..num_latency_pings {
            // Handle the ping response
            let ping_request_counter = latency_info_state.get_and_increment_latency_ping_counter();
            let latency_ping_duration = ping_request_counter as f64;
            handle_monitoring_service_response(
                &mut latency_info_state,
                ping_request_counter,
                ping_request_counter,
                latency_ping_duration,
            );

            // Verify the number of recorded latencies
            let recorded_latency_pings = latency_info_state.get_recorded_latency_pings();
            let expected_num_latency_pings = min(max_num_latency_pings_to_retain, i + 1);
            assert_eq!(
                recorded_latency_pings.len() as u64,
                expected_num_latency_pings,
            );

            // Verify the recorded latencies
            let lowest_latency_ping_counter =
                if ping_request_counter >= max_num_latency_pings_to_retain {
                    ping_request_counter - max_num_latency_pings_to_retain + 1
                } else {
                    ping_request_counter
                };
            for latency_ping_counter in lowest_latency_ping_counter..ping_request_counter + 1 {
                let latency_ping_duration =
                    recorded_latency_pings.get(&latency_ping_counter).unwrap();
                assert_eq!(*latency_ping_duration, latency_ping_counter as f64);
            }

            // Verify the average latency
            assert_eq!(
                latency_info_state.get_average_latency_ping_secs().unwrap(),
                recorded_latency_pings.values().sum::<f64>() / recorded_latency_pings.len() as f64,
            );
        }
    }

    /// Returns a random U64
    fn get_random_u64() -> u64 {
        let mut rng = OsRng;
        rng.r#gen()
    }

    /// Handles a monitoring service response from a peer
    fn handle_monitoring_service_response(
        latency_info_state: &mut LatencyInfoState,
        request_ping_counter: u64,
        response_ping_counter: u64,
        response_time_secs: f64,
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
            PeerMonitoringServiceRequest::LatencyPing(LatencyPingRequest {
                ping_counter: request_ping_counter,
            });

        // Create the service response
        let peer_monitoring_service_response =
            PeerMonitoringServiceResponse::LatencyPing(LatencyPingResponse {
                ping_counter: response_ping_counter,
            });

        // Handle the response
        latency_info_state.handle_monitoring_service_response(
            &peer_network_id,
            peer_metadata,
            peer_monitoring_service_request,
            peer_monitoring_service_response,
            response_time_secs,
        );
    }

    /// Verifies that there are no recorded latency pings
    fn verify_no_recorded_pings(latency_info_state: &mut LatencyInfoState) {
        assert!(latency_info_state
            .recorded_latency_ping_durations_secs
            .is_empty());
    }
}
