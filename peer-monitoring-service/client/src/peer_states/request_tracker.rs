// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_time_service::{TimeService, TimeServiceTrait};
use std::{
    ops::Add,
    time::{Duration, Instant},
};

/// A simple container that tracks request and response states
#[derive(Clone, Debug)]
pub struct RequestTracker {
    in_flight_request: bool, // If there is a request currently in-flight
    last_request_time: Option<Instant>, // The most recent request time
    last_response_time: Option<Instant>, // The most recent response time
    num_consecutive_request_failures: u64, // The number of consecutive request failures
    request_interval_usec: u64, // The interval (usec) between requests
    time_service: TimeService, // The time service to use for duration calculation
}

impl RequestTracker {
    /// Creates a new request tracker with the given request interval in ms
    pub fn new(request_interval_ms: u64, time_service: TimeService) -> Self {
        let request_interval_usec = request_interval_ms * 1000;
        RequestTracker::new_with_microseconds(request_interval_usec, time_service)
    }

    /// Creates a new request tracker with the given request interval in usec
    pub fn new_with_microseconds(request_interval_usec: u64, time_service: TimeService) -> Self {
        Self {
            in_flight_request: false,
            last_request_time: None,
            last_response_time: None,
            num_consecutive_request_failures: 0,
            request_interval_usec,
            time_service,
        }
    }

    /// Returns the last request time
    pub fn get_last_request_time(&self) -> Option<Instant> {
        self.last_request_time
    }

    /// Returns the last response time
    pub fn get_last_response_time(&self) -> Option<Instant> {
        self.last_response_time
    }

    /// Returns the number of consecutive failures
    pub fn get_num_consecutive_failures(&self) -> u64 {
        self.num_consecutive_request_failures
    }

    /// Returns true iff there is a request currently in-flight
    pub fn in_flight_request(&self) -> bool {
        self.in_flight_request
    }

    /// Updates the state to mark a request as having started
    pub fn request_started(&mut self) {
        // Mark the request as in-flight
        self.in_flight_request = true;

        // Update the last request time
        self.last_request_time = Some(self.time_service.now());
    }

    /// Updates the state to mark a request as having completed
    pub fn request_completed(&mut self) {
        self.in_flight_request = false;
    }

    /// Returns true iff a new request should be sent (based
    /// on the latest response time).
    pub fn new_request_required(&self) -> bool {
        // There's already an in-flight request. A new one should not be sent.
        if self.in_flight_request() {
            return false;
        }

        // Otherwise, check the last request time for freshness
        match self.last_request_time {
            Some(last_request_time) => {
                self.time_service.now()
                    > last_request_time.add(Duration::from_micros(self.request_interval_usec))
            },
            None => true, // A request should be sent immediately
        }
    }

    /// Records a successful response for the request
    pub fn record_response_success(&mut self) {
        // Update the last response time
        self.last_response_time = Some(self.time_service.now());

        // Reset the number of consecutive failures
        self.num_consecutive_request_failures = 0;
    }

    /// Records a failure for the request
    pub fn record_response_failure(&mut self) {
        self.num_consecutive_request_failures += 1;
    }
}

#[cfg(test)]
mod test {
    use crate::peer_states::request_tracker::RequestTracker;
    use velor_time_service::{TimeService, TimeServiceTrait};
    use std::time::Duration;

    #[test]
    fn test_simple_request_flow() {
        // Create the request tracker
        let request_interval_ms = 100;
        let time_service = TimeService::mock();
        let mut request_tracker = RequestTracker::new(request_interval_ms, time_service.clone());

        // Verify no requests have been sent
        assert!(request_tracker.get_last_request_time().is_none());
        assert!(request_tracker.get_last_response_time().is_none());
        assert_eq!(request_tracker.get_num_consecutive_failures(), 0);

        // Emulate several request flows
        let mock_time = time_service.into_mock();
        for _ in 0..10 {
            // Verify there are no in-flight requests and that a new request is required
            assert!(!request_tracker.in_flight_request());
            assert!(request_tracker.new_request_required());

            // Mark a request as having started and verify the state of the tracker
            let request_time = mock_time.now();
            request_tracker.request_started();
            assert!(request_tracker.in_flight_request());
            assert_eq!(request_tracker.get_last_request_time(), Some(request_time));
            assert!(!request_tracker.new_request_required());

            // Mark a request as having completed and verify the state of the tracker
            request_tracker.request_completed();
            assert!(!request_tracker.in_flight_request());
            assert!(!request_tracker.new_request_required());

            // Elapse a little time, record a successful response and verify the
            // state of the tracker (the number of consecutive failures should reset).
            mock_time.advance(Duration::from_millis(request_interval_ms / 2));
            let response_time = mock_time.now();
            request_tracker.record_response_success();
            assert_eq!(request_tracker.get_last_request_time(), Some(request_time));
            assert_eq!(
                request_tracker.get_last_response_time(),
                Some(response_time)
            );

            // Verify a new request is not required because we need more time to elapse
            assert!(!request_tracker.new_request_required());
            assert_eq!(request_tracker.get_num_consecutive_failures(), 0);

            // Elapse more time and verify a new request is now required
            mock_time.advance(Duration::from_millis((request_interval_ms / 2) + 1));
            assert!(request_tracker.new_request_required());
        }
    }

    #[test]
    fn test_response_failures() {
        // Create the request tracker
        let request_interval_ms = 100;
        let time_service = TimeService::mock();
        let mut request_tracker = RequestTracker::new(request_interval_ms, time_service.clone());

        // Emulate several failure flows
        let mock_time = time_service.into_mock();
        let num_failures = 10;
        for i in 0..num_failures {
            // Mark a request as having started
            let request_time = mock_time.now();
            request_tracker.request_started();

            // Mark a request as having completed
            request_tracker.request_completed();

            // Record a failure response and verify the state of the tracker
            request_tracker.record_response_failure();
            assert_eq!(request_tracker.get_last_request_time(), Some(request_time));
            assert!(request_tracker.get_last_response_time().is_none());
            assert_eq!(request_tracker.get_num_consecutive_failures(), i + 1);

            // Elapse more time and verify a new request is now required
            mock_time.advance(Duration::from_millis(request_interval_ms + 1));
            assert!(request_tracker.new_request_required());
        }

        // Verify the number of failures
        assert_eq!(request_tracker.get_num_consecutive_failures(), num_failures);

        // Mark a request as having started
        let request_time = mock_time.now();
        request_tracker.request_started();

        // Mark a request as having completed
        request_tracker.request_completed();

        // Elapse a little time, record a successful response and verify the state of the tracker
        mock_time.advance(Duration::from_millis(request_interval_ms / 2));
        let response_time = mock_time.now();
        request_tracker.record_response_success();
        assert_eq!(request_tracker.get_last_request_time(), Some(request_time));
        assert_eq!(
            request_tracker.get_last_response_time(),
            Some(response_time)
        );
        assert_eq!(request_tracker.get_num_consecutive_failures(), 0);

        // Emulate several additional failure flows
        for i in 0..num_failures {
            // Mark a request as having started
            let request_time = mock_time.now();
            request_tracker.request_started();

            // Mark a request as having completed
            request_tracker.request_completed();

            // Record a failure response and verify the state of the tracker
            request_tracker.record_response_failure();
            assert_eq!(request_tracker.get_last_request_time(), Some(request_time));
            assert_eq!(
                request_tracker.get_last_response_time(),
                Some(response_time)
            );
            assert_eq!(request_tracker.get_num_consecutive_failures(), i + 1);

            // Elapse more time and verify a new request is now required
            mock_time.advance(Duration::from_millis(request_interval_ms + 1));
            assert!(request_tracker.new_request_required());
        }
    }
}
