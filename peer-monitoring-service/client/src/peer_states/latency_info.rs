// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    metrics,
    peer_states::{key_value::StateValueInterface, request_tracker::RequestTracker},
    Error, LogEntry, LogEvent, LogSchema, PeerMonitoringServiceClient,
};
use aptos_config::{config::LatencyMonitoringConfig, network_id::PeerNetworkId};
use aptos_infallible::RwLock;
use aptos_logger::{error, info, warn};
use aptos_network::{
    application::{
        interface::{NetworkClient, NetworkClientInterface},
        metadata::PeerMetadata,
    },
    peer::DisconnectReason,
};
use aptos_peer_monitoring_service_types::{
    request::{LatencyPingRequest, PeerMonitoringServiceRequest},
    response::PeerMonitoringServiceResponse,
    PeerMonitoringServiceMessage,
};
use aptos_time_service::TimeService;
use std::{
    collections::BTreeMap,
    fmt,
    fmt::{Display, Formatter},
    sync::Arc,
    time::Duration,
};

// The timeout for disconnecting from a peer after too many ping failures
const DISCONNECT_TIMEOUT_SECS: u64 = 1;

/// A simple container that holds a peer's latency info
#[derive(Clone, Debug)]
pub struct LatencyInfoState<
    T: NetworkClientInterface<PeerMonitoringServiceMessage> + 'static = NetworkClient<
        PeerMonitoringServiceMessage,
    >,
> {
    latency_monitoring_config: LatencyMonitoringConfig, // The config for latency monitoring
    latency_ping_counter: u64, // The monotonically increasing counter for each ping
    peer_monitoring_client: PeerMonitoringServiceClient<T>, // The network client for disconnects
    recorded_latency_ping_durations_secs: BTreeMap<u64, f64>, // Successful ping durations by counter (secs)
    request_tracker: Arc<RwLock<RequestTracker>>, // The request tracker for latency ping requests
}

impl<T: NetworkClientInterface<PeerMonitoringServiceMessage> + 'static> LatencyInfoState<T> {
    pub fn new(
        latency_monitoring_config: LatencyMonitoringConfig,
        peer_monitoring_client: PeerMonitoringServiceClient<T>,
        time_service: TimeService,
    ) -> Self {
        let request_tracker = RequestTracker::new(
            latency_monitoring_config.latency_ping_interval_ms,
            time_service,
        );

        Self {
            latency_monitoring_config,
            latency_ping_counter: 0,
            peer_monitoring_client,
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

    /// Handles a ping failure for the specified peer. If too many consecutive
    /// failures occur and disconnect is enabled, disconnects from the peer.
    fn handle_request_failure(&self, peer_network_id: &PeerNetworkId) {
        // Update the number of ping failures for the request tracker
        self.request_tracker.write().record_response_failure();

        // Check if the number of ping failures is too high
        let num_consecutive_failures = self.request_tracker.read().get_num_consecutive_failures();
        if num_consecutive_failures >= self.latency_monitoring_config.max_latency_ping_failures {
            // Check if we should disconnect from the peer
            if self
                .latency_monitoring_config
                .disconnect_from_peers_on_failures
            {
                warn!(LogSchema::new(LogEntry::LatencyPing)
                    .event(LogEvent::TooManyPingFailures)
                    .peer(peer_network_id)
                    .message("Too many ping failures occurred for the peer! Disconnecting."));

                // Spawn a task to disconnect from the peer (asynchronously, to avoid blocking)
                let peer_monitoring_client = self.peer_monitoring_client.clone();
                let peer_network_id = *peer_network_id;
                tokio::spawn(async move {
                    disconnect_from_peer(peer_monitoring_client, peer_network_id).await;
                });
            } else {
                warn!(LogSchema::new(LogEntry::LatencyPing)
                    .event(LogEvent::TooManyPingFailures)
                    .peer(peer_network_id)
                    .message("Too many ping failures for the peer, but disconnect is disabled."));
            }
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

/// Disconnects from the specified peer due to too many ping failures
async fn disconnect_from_peer<T: NetworkClientInterface<PeerMonitoringServiceMessage>>(
    peer_monitoring_client: PeerMonitoringServiceClient<T>,
    peer_network_id: PeerNetworkId,
) {
    // Log the disconnect attempt
    info!(LogSchema::new(LogEntry::LatencyPing)
        .event(LogEvent::TooManyPingFailures)
        .peer(&peer_network_id)
        .message("Disconnecting from peer due to too many ping failures"));

    // Disconnect from the peer with a timeout (to prevent hanging indefinitely)
    let disconnect_result = tokio::time::timeout(
        Duration::from_secs(DISCONNECT_TIMEOUT_SECS),
        peer_monitoring_client
            .disconnect_from_peer(peer_network_id, DisconnectReason::PeerMonitoringPingFailure),
    )
    .await;

    // Log any errors
    match disconnect_result {
        Ok(Ok(())) => {
            info!(LogSchema::new(LogEntry::LatencyPing)
                .event(LogEvent::TooManyPingFailures)
                .peer(&peer_network_id)
                .message("Successfully disconnected from peer!"));
        },
        Ok(Err(error)) => {
            warn!(LogSchema::new(LogEntry::LatencyPing)
                .event(LogEvent::TooManyPingFailures)
                .peer(&peer_network_id)
                .message(&format!(
                    "Failed to disconnect from peer! Error: {:?}",
                    error
                )));
        },
        Err(error) => {
            warn!(LogSchema::new(LogEntry::LatencyPing)
                .event(LogEvent::TooManyPingFailures)
                .peer(&peer_network_id)
                .message(&format!(
                    "Timeout while disconnecting from peer! Error: {:?}",
                    error
                )));
        },
    }
}

impl<T: NetworkClientInterface<PeerMonitoringServiceMessage>> StateValueInterface
    for LatencyInfoState<T>
{
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
        // Log the error
        warn!(LogSchema::new(LogEntry::LatencyPing)
            .event(LogEvent::ResponseError)
            .message("Error encountered when pinging peer!")
            .peer(peer_network_id)
            .error(&error));

        // Handle the failure
        self.handle_request_failure(peer_network_id);
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

impl<T: NetworkClientInterface<PeerMonitoringServiceMessage>> Display for LatencyInfoState<T> {
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
    use crate::{
        peer_states::{
            key_value::StateValueInterface,
            latency_info::{LatencyInfoState, DISCONNECT_TIMEOUT_SECS},
        },
        Error, PeerMonitoringServiceClient,
    };
    use aptos_config::{
        config::{LatencyMonitoringConfig, PeerRole},
        network_id::{NetworkId, PeerNetworkId},
    };
    use aptos_infallible::RwLock;
    use aptos_netcore::transport::ConnectionOrigin;
    use aptos_network::{
        application::{
            interface::NetworkClientInterface, metadata::PeerMetadata, storage::PeersAndMetadata,
        },
        peer::DisconnectReason,
        protocols::wire::handshake::v1::{MessagingProtocolVersion, ProtocolIdSet},
        transport::{ConnectionId, ConnectionMetadata},
    };
    use aptos_peer_monitoring_service_types::{
        request::{LatencyPingRequest, PeerMonitoringServiceRequest},
        response::{LatencyPingResponse, PeerMonitoringServiceResponse},
        PeerMonitoringServiceMessage,
    };
    use aptos_time_service::TimeService;
    use aptos_types::{network_address::NetworkAddress, PeerId};
    use async_trait::async_trait;
    use bytes::Bytes;
    use rand::{rngs::OsRng, Rng};
    use std::{cmp::min, collections::HashMap, str::FromStr, sync::Arc, time::Duration};
    use tokio::time::sleep;

    // Useful test constants
    const TEST_NETWORK_ADDRESS: &str = "/ip4/127.0.0.1/tcp/8081";

    /// A mock network client that tracks disconnect calls for testing
    #[derive(Clone, Debug)]
    struct MockNetworkClient {
        /// Tracks all disconnect calls: (peer_network_id, disconnect_reason)
        disconnect_calls: Arc<RwLock<Vec<(PeerNetworkId, DisconnectReason)>>>,
        /// The peers and metadata container
        peers_and_metadata: Arc<PeersAndMetadata>,
    }

    impl MockNetworkClient {
        fn new() -> Self {
            Self {
                disconnect_calls: Arc::new(RwLock::new(Vec::new())),
                peers_and_metadata: PeersAndMetadata::new(&[NetworkId::Validator]),
            }
        }

        /// Returns the number of disconnect calls made
        fn get_disconnect_call_count(&self) -> usize {
            self.disconnect_calls.read().len()
        }

        /// Returns the disconnect calls made
        fn get_disconnect_calls(&self) -> Vec<(PeerNetworkId, DisconnectReason)> {
            self.disconnect_calls.read().clone()
        }
    }

    #[async_trait]
    impl NetworkClientInterface<PeerMonitoringServiceMessage> for MockNetworkClient {
        async fn add_peers_to_discovery(
            &self,
            _peers: &[(PeerNetworkId, NetworkAddress)],
        ) -> Result<(), aptos_network::application::error::Error> {
            Ok(())
        }

        async fn disconnect_from_peer(
            &self,
            peer: PeerNetworkId,
            disconnect_reason: DisconnectReason,
        ) -> Result<(), aptos_network::application::error::Error> {
            self.disconnect_calls
                .write()
                .push((peer, disconnect_reason));
            Ok(())
        }

        fn get_available_peers(
            &self,
        ) -> Result<Vec<PeerNetworkId>, aptos_network::application::error::Error> {
            Ok(vec![])
        }

        fn get_peers_and_metadata(&self) -> Arc<PeersAndMetadata> {
            self.peers_and_metadata.clone()
        }

        fn send_to_peer(
            &self,
            _message: PeerMonitoringServiceMessage,
            _peer: PeerNetworkId,
        ) -> Result<(), aptos_network::application::error::Error> {
            Ok(())
        }

        fn send_to_peer_raw(
            &self,
            _message: Bytes,
            _peer: PeerNetworkId,
        ) -> Result<(), aptos_network::application::error::Error> {
            Ok(())
        }

        fn send_to_peers(
            &self,
            _message: PeerMonitoringServiceMessage,
            _peers: Vec<PeerNetworkId>,
        ) -> Result<(), aptos_network::application::error::Error> {
            Ok(())
        }

        async fn send_to_peer_rpc(
            &self,
            _message: PeerMonitoringServiceMessage,
            _rpc_timeout: Duration,
            _peer: PeerNetworkId,
        ) -> Result<PeerMonitoringServiceMessage, aptos_network::application::error::Error>
        {
            Err(aptos_network::application::error::Error::UnexpectedError(
                "Not implemented".to_string(),
            ))
        }

        async fn send_to_peer_rpc_raw(
            &self,
            _message: Bytes,
            _rpc_timeout: Duration,
            _peer: PeerNetworkId,
        ) -> Result<PeerMonitoringServiceMessage, aptos_network::application::error::Error>
        {
            Err(aptos_network::application::error::Error::UnexpectedError(
                "Not implemented".to_string(),
            ))
        }

        fn to_bytes_by_protocol(
            &self,
            _peers: Vec<PeerNetworkId>,
            _message: PeerMonitoringServiceMessage,
        ) -> anyhow::Result<HashMap<PeerNetworkId, Bytes>> {
            Ok(HashMap::new())
        }

        fn sort_peers_by_latency(&self, _network: NetworkId, _peers: &mut [PeerId]) {
            // No-op for mock
        }
    }

    #[test]
    fn test_verify_latency_info_state() {
        // Create the latency info state
        let latency_monitoring_config = LatencyMonitoringConfig::default();
        let (peer_monitoring_client, _) = create_mock_peer_monitoring_client();
        let time_service = TimeService::mock();
        let mut latency_info_state = LatencyInfoState::new(
            latency_monitoring_config,
            peer_monitoring_client,
            time_service,
        );

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
        let (peer_monitoring_client, _) = create_mock_peer_monitoring_client();
        let time_service = TimeService::mock();
        let mut latency_info_state = LatencyInfoState::new(
            latency_monitoring_config,
            peer_monitoring_client,
            time_service,
        );

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

    #[tokio::test]
    async fn test_disconnect_called_on_max_failures() {
        // Create a latency monitoring config (with disconnect enabled)
        let latency_monitoring_config = LatencyMonitoringConfig {
            disconnect_from_peers_on_failures: true,
            max_latency_ping_failures: 3,
            ..LatencyMonitoringConfig::default()
        };

        // Create a latency info state with the mock network client
        let (peer_monitoring_client, mock_network_client) = create_mock_peer_monitoring_client();
        let mut latency_info_state = LatencyInfoState::new(
            latency_monitoring_config,
            peer_monitoring_client,
            TimeService::mock(),
        );

        // Verify no disconnect calls initially
        assert_eq!(mock_network_client.get_disconnect_call_count(), 0);

        // Create a test peer
        let peer_network_id = PeerNetworkId::new(NetworkId::Validator, PeerId::random());

        // Trigger failures up to but not exceeding the threshold
        trigger_ping_failures(
            &mut latency_info_state,
            &peer_network_id,
            latency_monitoring_config.max_latency_ping_failures - 1,
        );

        // Give the disconnect task time to complete
        sleep(Duration::from_secs(DISCONNECT_TIMEOUT_SECS)).await;

        // Verify no disconnect was called yet (below threshold)
        assert_eq!(mock_network_client.get_disconnect_call_count(), 0);

        // Trigger one more failure to exceed the threshold
        trigger_ping_failures(&mut latency_info_state, &peer_network_id, 1);

        // Give the disconnect task time to complete
        sleep(Duration::from_secs(DISCONNECT_TIMEOUT_SECS)).await;

        // Verify disconnect was called
        assert_eq!(mock_network_client.get_disconnect_call_count(), 1);

        // Verify the disconnect was for the correct peer and reason
        let disconnect_calls = mock_network_client.get_disconnect_calls();
        let (called_peer_network_id, disconnect_reason) = disconnect_calls.first().unwrap();
        assert_eq!(called_peer_network_id, &peer_network_id);
        assert!(matches!(
            disconnect_reason,
            DisconnectReason::PeerMonitoringPingFailure
        ));
    }

    #[tokio::test]
    async fn test_disconnect_not_called_when_disabled() {
        // Create a latency monitoring config (with disconnect disabled)
        let latency_monitoring_config = LatencyMonitoringConfig {
            disconnect_from_peers_on_failures: false,
            max_latency_ping_failures: 3,
            ..LatencyMonitoringConfig::default()
        };

        // Create a latency info state with the mock network client
        let (peer_monitoring_client, mock_network_client) = create_mock_peer_monitoring_client();
        let mut latency_info_state = LatencyInfoState::new(
            latency_monitoring_config,
            peer_monitoring_client,
            TimeService::mock(),
        );

        // Create a test peer
        let peer_network_id = PeerNetworkId::new(NetworkId::Validator, PeerId::random());

        // Trigger failures exceeding the threshold
        trigger_ping_failures(
            &mut latency_info_state,
            &peer_network_id,
            latency_monitoring_config.max_latency_ping_failures + 5,
        );

        // Give the disconnect task time to complete
        sleep(Duration::from_secs(DISCONNECT_TIMEOUT_SECS)).await;

        // Verify disconnect was not called (config is disabled)
        assert_eq!(mock_network_client.get_disconnect_call_count(), 0);
    }

    #[tokio::test]
    async fn test_disconnect_called_multiple_times_for_continued_failures() {
        // Create a latency monitoring config (with disconnect enabled)
        let latency_monitoring_config = LatencyMonitoringConfig {
            disconnect_from_peers_on_failures: true,
            max_latency_ping_failures: 2,
            ..LatencyMonitoringConfig::default()
        };

        // Create a latency info state with the mock network client
        let (peer_monitoring_client, mock_network_client) = create_mock_peer_monitoring_client();
        let mut latency_info_state = LatencyInfoState::new(
            latency_monitoring_config,
            peer_monitoring_client,
            TimeService::mock(),
        );

        // Create a test peer
        let peer_network_id = PeerNetworkId::new(NetworkId::Validator, PeerId::random());

        // Trigger failures exceeding the threshold multiple times
        trigger_ping_failures(&mut latency_info_state, &peer_network_id, 2);
        sleep(Duration::from_secs(DISCONNECT_TIMEOUT_SECS)).await;
        assert_eq!(mock_network_client.get_disconnect_call_count(), 1);

        // Continued failures should trigger more disconnects
        trigger_ping_failures(&mut latency_info_state, &peer_network_id, 1);
        sleep(Duration::from_secs(DISCONNECT_TIMEOUT_SECS)).await;
        assert_eq!(mock_network_client.get_disconnect_call_count(), 2);

        // Additional failures should continue to trigger disconnects
        trigger_ping_failures(&mut latency_info_state, &peer_network_id, 1);
        sleep(Duration::from_secs(DISCONNECT_TIMEOUT_SECS)).await;
        assert_eq!(mock_network_client.get_disconnect_call_count(), 3);
    }

    #[tokio::test]
    async fn test_success_resets_failure_counter_no_disconnect() {
        // Create a latency monitoring config (with disconnect enabled)
        let latency_monitoring_config = LatencyMonitoringConfig {
            disconnect_from_peers_on_failures: true,
            max_latency_ping_failures: 3,
            ..LatencyMonitoringConfig::default()
        };

        // Create a latency info state with the mock network client
        let (peer_monitoring_client, mock_network_client) = create_mock_peer_monitoring_client();
        let mut latency_info_state = LatencyInfoState::new(
            latency_monitoring_config,
            peer_monitoring_client,
            TimeService::mock(),
        );

        // Create a test peer
        let peer_network_id = PeerNetworkId::new(NetworkId::Validator, PeerId::random());

        // Trigger failures up to but not exceeding the threshold
        trigger_ping_failures(
            &mut latency_info_state,
            &peer_network_id,
            latency_monitoring_config.max_latency_ping_failures - 1,
        );
        sleep(Duration::from_secs(DISCONNECT_TIMEOUT_SECS)).await;

        // Verify no disconnect was called
        assert_eq!(mock_network_client.get_disconnect_call_count(), 0);

        // Record a successful ping (this resets the failure counter)
        let ping_counter = latency_info_state.get_and_increment_latency_ping_counter();
        latency_info_state.record_new_latency_and_reset_failures(ping_counter, 0.1);

        // Trigger more failures, but less than the threshold
        trigger_ping_failures(
            &mut latency_info_state,
            &peer_network_id,
            latency_monitoring_config.max_latency_ping_failures - 1,
        );
        sleep(Duration::from_secs(DISCONNECT_TIMEOUT_SECS)).await;

        // Verify disconnect was still not called (counter was reset)
        assert_eq!(mock_network_client.get_disconnect_call_count(), 0);
    }

    /// Creates a peer monitoring service client with a mock network client
    fn create_mock_peer_monitoring_client() -> (
        PeerMonitoringServiceClient<MockNetworkClient>,
        MockNetworkClient,
    ) {
        let mock_network_client = MockNetworkClient::new();
        let peer_monitoring_client = PeerMonitoringServiceClient::new(mock_network_client.clone());
        (peer_monitoring_client, mock_network_client)
    }

    /// Returns a random U64
    fn get_random_u64() -> u64 {
        let mut rng = OsRng;
        rng.r#gen()
    }

    /// Handles a monitoring service response from a peer
    fn handle_monitoring_service_response(
        latency_info_state: &mut LatencyInfoState<MockNetworkClient>,
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

    /// Triggers multiple ping failures for the given latency info state
    fn trigger_ping_failures<T: NetworkClientInterface<PeerMonitoringServiceMessage> + 'static>(
        latency_info_state: &mut LatencyInfoState<T>,
        peer_network_id: &PeerNetworkId,
        num_failures: u64,
    ) {
        for _ in 0..num_failures {
            latency_info_state.handle_monitoring_service_response_error(
                peer_network_id,
                Error::UnexpectedError("Test ping failure".to_string()),
            );
        }
    }

    /// Verifies that there are no recorded latency pings
    fn verify_no_recorded_pings(latency_info_state: &mut LatencyInfoState<MockNetworkClient>) {
        assert!(latency_info_state
            .recorded_latency_ping_durations_secs
            .is_empty());
    }
}
