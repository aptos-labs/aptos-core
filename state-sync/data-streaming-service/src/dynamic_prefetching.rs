// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics, stream_engine::StreamEngine};
use aptos_config::config::{DataStreamingServiceConfig, DynamicPrefetchingConfig};
use aptos_time_service::{TimeService, TimeServiceTrait};
use std::{
    cmp::{max, min},
    time::{Duration, Instant},
};

/// A simple container for the dynamic prefetching state
#[derive(Debug)]
pub struct DynamicPrefetchingState {
    // The data streaming service config
    streaming_service_config: DataStreamingServiceConfig,

    // The instant the last timeout occurred (if any)
    last_timeout_instant: Option<Instant>,

    // The maximum number of concurrent requests that can be executing at any given time
    max_dynamic_concurrent_requests: u64,

    // The time service to track elapsed time (e.g., during stream lag checks)
    time_service: TimeService,
}

impl DynamicPrefetchingState {
    pub fn new(
        data_streaming_service_config: DataStreamingServiceConfig,
        time_service: TimeService,
    ) -> Self {
        // Get the initial prefetching value from the config
        let max_dynamic_concurrent_requests = data_streaming_service_config
            .dynamic_prefetching
            .initial_prefetching_value;

        // Create and return the new dynamic prefetching state
        Self {
            streaming_service_config: data_streaming_service_config,
            last_timeout_instant: None,
            max_dynamic_concurrent_requests,
            time_service,
        }
    }

    /// A simple helper function that returns the dynamic prefetching config
    fn get_dynamic_prefetching_config(&self) -> &DynamicPrefetchingConfig {
        &self.streaming_service_config.dynamic_prefetching
    }

    /// Returns true iff dynamic prefetching is enabled
    fn is_dynamic_prefetching_enabled(&self) -> bool {
        self.get_dynamic_prefetching_config()
            .enable_dynamic_prefetching
    }

    /// Returns true iff the prefetching value is currently frozen (i.e.,
    /// to avoid overly increasing the value near saturation). Freezing
    /// occurs after a timeout and lasts for a configured duration.
    fn is_prefetching_value_frozen(&self) -> bool {
        match self.last_timeout_instant {
            Some(last_failure_time) => {
                // Get the time since the last failure and max freeze duration
                let time_since_last_failure =
                    self.time_service.now().duration_since(last_failure_time);
                let max_freeze_duration = Duration::from_secs(
                    self.get_dynamic_prefetching_config()
                        .timeout_freeze_duration_secs,
                );

                // Check if the time since the last failure is less than the freeze duration
                time_since_last_failure < max_freeze_duration
            },
            None => false, // No failures have occurred
        }
    }

    /// Returns the number of maximum concurrent requests that can be executing
    /// at any given time. Depending on if dynamic prefetching is enabled, this
    /// value will be dynamic or static (i.e., config defined).
    pub fn get_max_concurrent_requests(&self, stream_engine: &StreamEngine) -> u64 {
        // If dynamic prefetching is disabled, use the static values defined
        // in the config. Otherwise get the current dynamic max value.
        let max_concurrent_requests = if !self.is_dynamic_prefetching_enabled() {
            match stream_engine {
                StreamEngine::StateStreamEngine(_) => {
                    // Use the configured max for state value requests
                    self.streaming_service_config.max_concurrent_state_requests
                },
                _ => {
                    // Use the configured max for all other requests
                    self.streaming_service_config.max_concurrent_requests
                },
            }
        } else {
            // Otherwise, return the current max value
            self.max_dynamic_concurrent_requests
        };

        // Update the metrics for the max concurrent requests
        metrics::set_max_concurrent_requests(max_concurrent_requests);

        max_concurrent_requests
    }

    /// Increases the maximum number of concurrent requests that should be executing.
    /// This is typically called after a successful response is received.
    pub fn increase_max_concurrent_requests(&mut self) {
        // If dynamic prefetching is disabled, or the value is currently frozen, do nothing
        if !self.is_dynamic_prefetching_enabled() || self.is_prefetching_value_frozen() {
            return;
        }

        // Otherwise, get and increase the current max
        let dynamic_prefetching_config = self.get_dynamic_prefetching_config();
        let amount_to_increase = dynamic_prefetching_config.prefetching_value_increase;
        let max_dynamic_concurrent_requests = self
            .max_dynamic_concurrent_requests
            .saturating_add(amount_to_increase);

        // Bound the value by the configured maximum
        let max_prefetching_value = dynamic_prefetching_config.max_prefetching_value;
        self.max_dynamic_concurrent_requests =
            min(max_dynamic_concurrent_requests, max_prefetching_value);
    }

    /// Decreases the maximum number of concurrent requests that should be executing.
    /// This is typically called after a timeout is received.
    pub fn decrease_max_concurrent_requests(&mut self) {
        // If dynamic prefetching is disabled, do nothing
        if !self.is_dynamic_prefetching_enabled() {
            return;
        }

        // Update the last failure time
        self.last_timeout_instant = Some(self.time_service.now());

        // Otherwise, get and decrease the current max
        let dynamic_prefetching_config = self.get_dynamic_prefetching_config();
        let amount_to_decrease = dynamic_prefetching_config.prefetching_value_decrease;
        let max_dynamic_concurrent_requests = self
            .max_dynamic_concurrent_requests
            .saturating_sub(amount_to_decrease);

        // Bound the value by the configured minimum
        let min_prefetching_value = dynamic_prefetching_config.min_prefetching_value;
        self.max_dynamic_concurrent_requests =
            max(max_dynamic_concurrent_requests, min_prefetching_value);
    }
}
