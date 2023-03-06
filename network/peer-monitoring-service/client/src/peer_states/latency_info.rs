// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    peer_states::{key_value::StateValueInterface, request_tracker::RequestTracker},
    Error, LogEntry, LogEvent, LogSchema,
};
use aptos_config::{config::LatencyMonitoringConfig, network_id::PeerNetworkId};
use aptos_infallible::RwLock;
use aptos_logger::{error, warn};
use aptos_network::application::metadata::PeerMetadata;
use aptos_peer_monitoring_service_types::{
    LatencyPingRequest, PeerMonitoringServiceRequest, PeerMonitoringServiceResponse,
};
use aptos_time_service::TimeService;
use std::{collections::BTreeMap, sync::Arc};

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
        &self,
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
}
