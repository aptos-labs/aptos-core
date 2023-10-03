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
use aptos_time_service::{TimeService, TimeServiceTrait};
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    fmt::{Display, Formatter},
    sync::Arc,
    time::Instant,
};

/// The maximum number of response entries to retain
const MAX_RESPONSE_ENTRIES_TO_RETAIN: usize = 10_000;

/// A simple container holding basic performance measurements for a peer
#[derive(Clone, Debug)]
pub struct PerformanceMeasurements {
    pub error_counts_by_type: HashMap<String, u64>, // The number of errors by error type
    pub max_response_entries_to_retain: usize,      // The max number of response entries to retain
    pub num_failed_responses: u64,                  // The number of failed responses
    pub num_successful_responses: u64,              // The number of successful responses
    pub response_data_size_bytes: u64,              // The size of the response data in bytes
    pub response_durations_sec: VecDeque<f64>, // Successful response durations (a simple FIFO queue)
    pub response_timestamps: VecDeque<Instant>, // The timestamps of the responses (a simple FIFO queue)
    pub time_service: TimeService,              // The time service to use for duration calculation
}

impl PerformanceMeasurements {
    pub fn new(
        response_data_size_bytes: u64,
        max_response_entries_to_retain: usize,
        time_service: TimeService,
    ) -> Self {
        Self {
            error_counts_by_type: HashMap::new(),
            max_response_entries_to_retain,
            num_failed_responses: 0,
            num_successful_responses: 0,
            response_data_size_bytes,
            response_durations_sec: VecDeque::new(),
            response_timestamps: VecDeque::new(),
            time_service,
        }
    }

    /// Handles a successful response with the given response time
    pub fn handle_successful_response(&mut self, response_time_secs: f64) -> Result<(), Error> {
        // Increase the number of successful responses
        self.num_successful_responses += 1;

        // Add the response duration and timestamp to the back of the queues
        self.response_durations_sec.push_back(response_time_secs);
        self.response_timestamps.push_back(self.time_service.now());

        // Garbage collect the queues if they are too large (by popping from the front)
        if self.response_durations_sec.len() > self.max_response_entries_to_retain {
            self.response_durations_sec.pop_front();
        }
        if self.response_timestamps.len() > self.max_response_entries_to_retain {
            self.response_timestamps.pop_front();
        }

        Ok(())
    }

    /// Handles a failed response with the given error
    pub fn handle_failed_response(&mut self, error: &Error) -> Result<(), Error> {
        // Increase the number of failed responses
        self.num_failed_responses += 1;

        // Increase the error count for the error type
        let error_label = error.get_label().to_string();
        let error_count = self.error_counts_by_type.entry(error_label).or_insert(0);
        *error_count += 1;

        Ok(())
    }

    /// Calculates the average response bandwidth (KB/s)
    fn calculate_average_response_bandwidth_kb(&self) -> Option<f64> {
        // Get the total duration between the first and last response
        let total_duration = self.calculate_total_response_duration_secs();

        // Calculate the average response bandwidth
        total_duration.map(|total_duration| {
            let response_data_size_bytes = self.response_data_size_bytes as f64;
            let num_responses = self.response_timestamps.len() as f64;
            let total_bandwidth_kb = (response_data_size_bytes * (num_responses - 1.0)) / 1024.0; // Convert to KB
            total_bandwidth_kb / total_duration
        })
    }

    /// Calculates the average response duration
    fn calculate_average_response_duration(&self) -> Option<f64> {
        let num_response_durations = self.response_durations_sec.len();
        if num_response_durations > 0 {
            let average_response_secs_sum: f64 = self.response_durations_sec.iter().sum();
            Some(average_response_secs_sum / num_response_durations as f64)
        } else {
            None
        }
    }

    /// Calculates the average number of responses per second
    fn calculate_average_responses_per_second(&self) -> Option<f64> {
        // Get the total duration between the first and last response
        let total_duration = self.calculate_total_response_duration_secs();

        // Calculate the average number of responses per second
        total_duration.map(|total_duration| {
            let num_responses = self.response_timestamps.len() as f64 - 1.0;
            num_responses / total_duration
        })
    }

    /// Calculates the duration between the first and last response (i.e.,
    /// the total response duration) in seconds.
    fn calculate_total_response_duration_secs(&self) -> Option<f64> {
        // Get the first and last timestamps
        let first_timestamp = self.response_timestamps.front().clone();
        let last_timestamp = self.response_timestamps.back().clone();

        // Calculate the duration between the timestamps
        match (first_timestamp, last_timestamp) {
            (Some(first_timestamp), Some(last_timestamp)) => Some(
                last_timestamp
                    .duration_since(*first_timestamp)
                    .as_secs_f64(),
            ),
            _ => None,
        }
    }
}

impl Display for PerformanceMeasurements {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "num_successful_responses: {}, num_failed_responses: {}, error_counts_by_type: {:?},\
            average_response_duration_sec: {:?}s, average_response_bandwidth: {:?} (KB/sec), \
            average_responses_per_second: {:?} (responses/sec)",
            self.num_successful_responses,
            self.num_failed_responses,
            self.error_counts_by_type,
            self.calculate_average_response_duration(),
            self.calculate_average_response_bandwidth_kb(),
            self.calculate_average_responses_per_second(),
        )
    }
}

/// A simple container that holds a single peer's performance monitoring state
#[derive(Clone, Debug)]
pub struct PerformanceMonitoringState {
    performance_measurements: PerformanceMeasurements, // The performance measurements for the peer
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
            RequestTracker::new_with_microseconds(request_interval_usecs, time_service.clone());

        Self {
            performance_measurements: PerformanceMeasurements::new(
                performance_monitoring_config.rpc_data_size,
                MAX_RESPONSE_ENTRIES_TO_RETAIN,
                time_service,
            ),
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
        response_time_secs: f64,
    ) {
        // Update the request tracker with a successful response
        self.request_tracker.write().record_response_success();

        // Save the response
        self.recorded_performance_response = Some(performance_response);

        // Update the performance measurements
        if let Err(error) = self
            .performance_measurements
            .handle_successful_response(response_time_secs)
        {
            error!(
                LogSchema::new(LogEntry::PerformanceMonitoringRequest).message(&format!(
                    "Failed to handle the successful response! Error: {}",
                    error
                ))
            );
        }
    }

    /// Handles a request failure for the specified peer
    fn handle_request_failure(&mut self, error: &Error) {
        // Update the request tracker
        self.request_tracker.write().record_response_failure();

        // Update the performance measurements
        if let Err(error) = self.performance_measurements.handle_failed_response(error) {
            error!(
                LogSchema::new(LogEntry::PerformanceMonitoringRequest).message(&format!(
                    "Failed to handle the request failure! Error: {}",
                    error
                ))
            );
        }
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
        response_time_secs: f64,
    ) {
        // Verify the request type is correctly formed
        let monitoring_service_request = match monitoring_service_request {
            PeerMonitoringServiceRequest::PerformanceMonitoringRequest(
                monitoring_service_request,
            ) => monitoring_service_request,
            request => {
                let error = Error::UnexpectedError(
                    "An unexpected request was sent instead of a performance monitoring request!"
                        .into(),
                );
                error!(LogSchema::new(LogEntry::SendRequest)
                    .error(&error)
                    .event(LogEvent::UnexpectedErrorEncountered)
                    .peer(peer_network_id)
                    .request(&request));
                self.handle_request_failure(&error);
                return;
            },
        };

        // Verify the response type is valid
        let performance_monitoring_response = match monitoring_service_response {
            PeerMonitoringServiceResponse::PerformanceMonitoring(
                performance_monitoring_response,
            ) => performance_monitoring_response,
            _ => {
                let error = Error::UnexpectedError("An unexpected response was received instead of a performance monitoring response!".into());
                warn!(LogSchema::new(LogEntry::PerformanceMonitoringRequest)
                    .error(&error)
                    .event(LogEvent::ResponseError)
                    .peer(peer_network_id));
                self.handle_request_failure(&error);
                return;
            },
        };

        // Verify the request counter is correct
        let request_counter = monitoring_service_request.request_counter;
        let response_counter = performance_monitoring_response.response_counter;
        if request_counter != response_counter {
            let error = Error::UnexpectedError(format!(
                "Peer responded with the incorrect request counter! Expected: {:?}, found: {:?}",
                request_counter, response_counter
            ));
            warn!(LogSchema::new(LogEntry::PerformanceMonitoringRequest)
                .error(&error)
                .event(LogEvent::ResponseError)
                .peer(peer_network_id));
            self.handle_request_failure(&error);
            return;
        }

        // Store the new performance response
        self.record_performance_monitoring_response(
            performance_monitoring_response,
            response_time_secs,
        );
    }

    fn handle_monitoring_service_response_error(
        &mut self,
        peer_network_id: &PeerNetworkId,
        error: Error,
    ) {
        // Handle the failure
        self.handle_request_failure(&error);

        // Log the error
        warn!(LogSchema::new(LogEntry::PerformanceMonitoringRequest)
            .error(&error)
            .event(LogEvent::ResponseError)
            .peer(peer_network_id)
            .error(&error));
    }

    fn update_peer_state_metrics(&self, _peer_network_id: &PeerNetworkId) {
        // TODO: Update the peer state metrics for performance monitoring.
        // This is currently a no-op.
    }
}

impl Display for PerformanceMonitoringState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PerformanceMonitoringState {{ request_counter: {}, recorded_performance_response: {:?}, performance_measurements: {} }}",
            self.request_counter,
            self.recorded_performance_response,
            self.performance_measurements
        )
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
    use crate::{
        error::Error,
        peer_states::{
            key_value::StateValueInterface,
            performance_monitoring::{PerformanceMeasurements, PerformanceMonitoringState},
        },
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
    use aptos_time_service::{TimeService, TimeServiceTrait};
    use aptos_types::{network_address::NetworkAddress, PeerId};
    use std::{ops::Add, str::FromStr, time::Duration};

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
            None,
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
                None,
            );
        }

        // Verify the performance monitoring state
        verify_performance_monitoring_state(&performance_monitoring_state, num_responses + 1);
    }

    #[test]
    fn test_verify_performance_measurement_state() {
        // Create the performance monitoring state
        let performance_monitoring_config = PerformanceMonitoringConfig::default();
        let time_service = TimeService::mock();
        let mut performance_monitoring_state =
            PerformanceMonitoringState::new(performance_monitoring_config, time_service.clone());

        // Verify the initial performance measurement state
        let performance_measurements = &performance_monitoring_state.performance_measurements;
        assert!(performance_measurements.error_counts_by_type.is_empty());
        assert_eq!(performance_measurements.num_failed_responses, 0);
        assert_eq!(performance_measurements.num_successful_responses, 0);
        assert_eq!(
            performance_measurements.response_data_size_bytes,
            performance_monitoring_config.rpc_data_size
        );
        assert!(performance_measurements.response_durations_sec.is_empty());
        assert!(performance_measurements.response_timestamps.is_empty());

        // Attempt to handle multiple invalid responses with mismatched request counters
        for i in 0..10 {
            // Handle the invalid monitoring response
            let request_counter = performance_monitoring_state.get_and_increment_request_counter();
            handle_monitoring_service_response(
                &mut performance_monitoring_state,
                request_counter,
                request_counter + 1,
                None,
            );

            // Verify the performance measurement state
            let performance_measurements = &performance_monitoring_state.performance_measurements;
            let error_counts_by_type = &performance_measurements.error_counts_by_type;
            assert_eq!(error_counts_by_type.len() as u64, 1);
            assert_eq!(*error_counts_by_type.values().next().unwrap(), i + 1);
            assert_eq!(performance_measurements.num_failed_responses, i + 1);
            assert_eq!(performance_measurements.num_successful_responses, 0);
            assert!(performance_measurements.response_durations_sec.is_empty());
            assert!(performance_measurements.response_timestamps.is_empty());
        }

        // Handle several valid monitoring responses
        let time_service = time_service.into_mock();
        let start_time = time_service.now();
        for i in 0..10 {
            // Elapse a second
            time_service.advance_ms(1000);

            // Handle the monitoring response
            let request_counter = performance_monitoring_state.get_and_increment_request_counter();
            let response_time_secs = Some(i as f64);
            handle_monitoring_service_response(
                &mut performance_monitoring_state,
                request_counter,
                request_counter,
                response_time_secs,
            );

            // Verify the number of failed and successful responses
            let performance_measurements = &performance_monitoring_state.performance_measurements;
            assert_eq!(performance_measurements.num_failed_responses, 10);
            assert_eq!(performance_measurements.num_successful_responses, i + 1);

            // Verify the response duration queue
            let response_durations_sec = &performance_measurements.response_durations_sec;
            assert_eq!(response_durations_sec.len() as u64, i + 1);
            assert_eq!(
                response_durations_sec.get(i as usize).unwrap().clone(),
                i as f64
            );

            // Verify the response timestamp queue
            let response_timestamps = &performance_measurements.response_timestamps;
            assert_eq!(response_timestamps.len() as u64, i + 1);
            assert_eq!(
                response_timestamps.get(i as usize).unwrap().clone(),
                start_time.add(Duration::from_secs(i + 1 as u64))
            );
        }
    }

    #[test]
    fn test_calculate_average_response_bandwidth_kb() {
        // Create the performance measurement state
        let time_service = TimeService::mock();
        let response_data_size_bytes = 2048; // 2 KB
        let max_response_entries_to_retain = 5;
        let mut performance_measurements = PerformanceMeasurements::new(
            response_data_size_bytes,
            max_response_entries_to_retain,
            time_service.clone(),
        );

        // Handle several valid monitoring responses
        let time_service = time_service.into_mock();
        for _ in 0..max_response_entries_to_retain {
            // Elapse a second
            time_service.advance_ms(1000);

            // Handle the monitoring response
            performance_measurements
                .handle_successful_response(0.0)
                .unwrap();
        }

        // Verify the average response bandwidth
        let total_data = response_data_size_bytes * (max_response_entries_to_retain - 1) as u64;
        let expected_bandwidth =
            (total_data as f64 / (max_response_entries_to_retain - 1) as f64) / 1024.0;
        assert_eq!(
            performance_measurements.calculate_average_response_bandwidth_kb(),
            Some(expected_bandwidth),
        );

        // Handle several invalid monitoring responses
        for _ in 0..3 {
            performance_measurements
                .handle_failed_response(&Error::NetworkError("Oops!".into()))
                .unwrap();
        }

        // Verify the average response bandwidth hasn't changed
        assert_eq!(
            performance_measurements.calculate_average_response_bandwidth_kb(),
            Some(expected_bandwidth),
        );

        // Handle several valid monitoring responses (more than the max response entries to retain)
        for _ in 1..=max_response_entries_to_retain {
            // Elapse two seconds (double the previous response time)
            time_service.advance_ms(2000);

            // Handle the monitoring response
            performance_measurements
                .handle_successful_response(0.0)
                .unwrap();
        }

        // Verify the new average response bandwidth is half the previous value
        assert_eq!(
            performance_measurements.calculate_average_response_bandwidth_kb(),
            Some(expected_bandwidth / 2.0)
        );
    }

    #[test]
    fn test_calculate_average_response_duration() {
        // Create the performance measurement state
        let max_response_entries_to_retain = 10;
        let mut performance_measurements = PerformanceMeasurements::new(
            2048, // 2 KB
            max_response_entries_to_retain,
            TimeService::mock(),
        );

        // Handle several valid monitoring responses
        for i in 1..=max_response_entries_to_retain {
            performance_measurements
                .handle_successful_response(i as f64)
                .unwrap();
        }

        // Verify the average response duration
        let total_duration = (1..=max_response_entries_to_retain).sum::<usize>() as f64;
        let average_duration = total_duration / max_response_entries_to_retain as f64;
        assert_eq!(
            performance_measurements.calculate_average_response_duration(),
            Some(average_duration),
        );

        // Handle several invalid monitoring responses
        for _ in 0..10 {
            performance_measurements
                .handle_failed_response(&Error::NetworkError("Oops!".into()))
                .unwrap();
        }

        // Verify the average response duration hasn't changed
        assert_eq!(
            performance_measurements.calculate_average_response_duration(),
            Some(average_duration),
        );

        // Handle several valid monitoring responses (more than the max response entries to retain)
        for i in 1..=max_response_entries_to_retain {
            performance_measurements
                .handle_successful_response(i as f64 * 10.0) // 10x the response duration
                .unwrap();
        }

        // Verify the new average response duration is 10x the previous value
        assert_eq!(
            performance_measurements.calculate_average_response_duration(),
            Some(average_duration * 10.0)
        );
    }

    #[test]
    fn test_calculate_average_responses_per_second() {
        // Create the performance measurement state
        let time_service = TimeService::mock();
        let max_response_entries_to_retain = 5;
        let mut performance_measurements = PerformanceMeasurements::new(
            1024, // 1 KB
            max_response_entries_to_retain,
            time_service.clone(),
        );

        // Handle several valid monitoring responses
        let time_service = time_service.into_mock();
        for _ in 0..max_response_entries_to_retain {
            // Elapse a second
            time_service.advance_ms(1000);

            // Handle the monitoring response
            performance_measurements
                .handle_successful_response(0.0)
                .unwrap();
        }

        // Verify the average responses per second is 1.0
        let expected_responses_per_second = 1.0;
        assert_eq!(
            performance_measurements.calculate_average_responses_per_second(),
            Some(expected_responses_per_second),
        );

        // Handle several invalid monitoring responses
        for _ in 0..10 {
            performance_measurements
                .handle_failed_response(&Error::NetworkError("Oops!".into()))
                .unwrap();
        }

        // Verify the average responses per second hasn't changed
        assert_eq!(
            performance_measurements.calculate_average_responses_per_second(),
            Some(expected_responses_per_second),
        );

        // Handle several valid monitoring responses (more than the max response entries to retain)
        for _ in 0..max_response_entries_to_retain {
            // Elapse two seconds (double the previous response time)
            time_service.advance_ms(2000);

            // Handle the monitoring response
            performance_measurements
                .handle_successful_response(0.0)
                .unwrap();
        }

        // Verify the new average responses per second is half the previous value
        assert_eq!(
            performance_measurements.calculate_average_responses_per_second(),
            Some(expected_responses_per_second / 2.0)
        );
    }

    #[test]
    fn test_calculate_total_response_duration_secs() {
        // Create the performance measurement state
        let time_service = TimeService::mock();
        let max_response_entries_to_retain = 5;
        let mut performance_measurements = PerformanceMeasurements::new(
            1024, // 1 KB
            max_response_entries_to_retain,
            time_service.clone(),
        );

        // Handle several valid monitoring responses
        let time_service = time_service.into_mock();
        for _ in 0..max_response_entries_to_retain {
            // Elapse a second
            time_service.advance_ms(1000);

            // Handle the monitoring response
            performance_measurements
                .handle_successful_response(0.0)
                .unwrap();
        }

        // Verify the total response duration
        let total_duration = max_response_entries_to_retain as f64 - 1.0;
        assert_eq!(
            performance_measurements.calculate_total_response_duration_secs(),
            Some(total_duration),
        );

        // Handle several invalid monitoring responses
        for _ in 0..10 {
            performance_measurements
                .handle_failed_response(&Error::NetworkError("Oops!".into()))
                .unwrap();
        }

        // Verify the total response duration hasn't changed
        assert_eq!(
            performance_measurements.calculate_total_response_duration_secs(),
            Some(total_duration),
        );

        // Handle several valid monitoring responses (more than the max response entries to retain)
        for _ in 0..max_response_entries_to_retain {
            // Elapse ten seconds (10x the previous response time)
            time_service.advance_ms(10_000);

            // Handle the monitoring response
            performance_measurements
                .handle_successful_response(0.0)
                .unwrap();
        }

        // Verify the new total response duration is 10x the previous value
        assert_eq!(
            performance_measurements.calculate_total_response_duration_secs(),
            Some(total_duration * 10.0)
        );
    }

    /// Handles a monitoring service response from a peer
    fn handle_monitoring_service_response(
        performance_monitoring_state: &mut PerformanceMonitoringState,
        request_counter: u64,
        response_counter: u64,
        response_time_secs: Option<f64>,
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
        let response_time_secs = response_time_secs.unwrap_or(0.0);
        performance_monitoring_state.handle_monitoring_service_response(
            &peer_network_id,
            peer_metadata,
            peer_monitoring_service_request,
            peer_monitoring_service_response,
            response_time_secs,
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
