// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics, stream_engine::StreamEngine};
use velor_config::config::{DataStreamingServiceConfig, DynamicPrefetchingConfig};
use velor_time_service::{TimeService, TimeServiceTrait};
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::streaming_client::{
        GetAllStatesRequest, GetAllTransactionsOrOutputsRequest, StreamRequest,
    };
    use velor_data_client::global_summary::AdvertisedData;

    #[test]
    fn test_initialize_prefetching_state() {
        // Create a data streaming service config with dynamic prefetching enabled
        let initial_prefetching_value = 5;
        let dynamic_prefetching_config = DynamicPrefetchingConfig {
            enable_dynamic_prefetching: true,
            initial_prefetching_value,
            ..Default::default()
        };
        let data_streaming_service_config = DataStreamingServiceConfig {
            dynamic_prefetching: dynamic_prefetching_config,
            ..Default::default()
        };

        // Create a new dynamic prefetching state
        let dynamic_prefetching_state =
            DynamicPrefetchingState::new(data_streaming_service_config, TimeService::mock());

        // Verify that the state was initialized correctly
        assert_eq!(
            dynamic_prefetching_state.streaming_service_config,
            data_streaming_service_config
        );
        assert_eq!(dynamic_prefetching_state.last_timeout_instant, None);
        assert_eq!(
            dynamic_prefetching_state.max_dynamic_concurrent_requests,
            initial_prefetching_value
        );
    }

    #[test]
    fn test_is_dynamic_prefetching_enabled() {
        // Create a data streaming service config with dynamic prefetching enabled
        let dynamic_prefetching_config = DynamicPrefetchingConfig {
            enable_dynamic_prefetching: true,
            ..Default::default()
        };
        let data_streaming_service_config = DataStreamingServiceConfig {
            dynamic_prefetching: dynamic_prefetching_config,
            ..Default::default()
        };

        // Create a new dynamic prefetching state
        let dynamic_prefetching_state =
            DynamicPrefetchingState::new(data_streaming_service_config, TimeService::mock());

        // Verify that dynamic prefetching is enabled
        assert!(dynamic_prefetching_state.is_dynamic_prefetching_enabled());

        // Create a data streaming service config with dynamic prefetching disabled
        let dynamic_prefetching_config = DynamicPrefetchingConfig {
            enable_dynamic_prefetching: false,
            ..Default::default()
        };
        let data_streaming_service_config = DataStreamingServiceConfig {
            dynamic_prefetching: dynamic_prefetching_config,
            ..Default::default()
        };

        // Create a new dynamic prefetching state
        let dynamic_prefetching_state =
            DynamicPrefetchingState::new(data_streaming_service_config, TimeService::mock());

        // Verify that dynamic prefetching is disabled
        assert!(!dynamic_prefetching_state.is_dynamic_prefetching_enabled());
    }

    #[test]
    fn test_is_prefetching_value_frozen() {
        // Create a data streaming service config with dynamic prefetching enabled
        let timeout_freeze_duration_secs = 10;
        let dynamic_prefetching_config = DynamicPrefetchingConfig {
            enable_dynamic_prefetching: true,
            timeout_freeze_duration_secs,
            ..Default::default()
        };
        let data_streaming_service_config = DataStreamingServiceConfig {
            dynamic_prefetching: dynamic_prefetching_config,
            ..Default::default()
        };

        // Create a new dynamic prefetching state
        let time_service = TimeService::mock();
        let mut dynamic_prefetching_state =
            DynamicPrefetchingState::new(data_streaming_service_config, time_service.clone());

        // Verify that the prefetching value is not frozen initially
        assert!(!dynamic_prefetching_state.is_prefetching_value_frozen());

        // Update the prefetcher state to simulate a timeout
        dynamic_prefetching_state.decrease_max_concurrent_requests();

        // Verify that the prefetching value is frozen
        assert!(dynamic_prefetching_state.is_prefetching_value_frozen());

        // Elapse less time than the freeze duration
        let time_service = time_service.into_mock();
        time_service.advance_secs(timeout_freeze_duration_secs - 1);

        // Verify that the prefetching value is still frozen
        assert!(dynamic_prefetching_state.is_prefetching_value_frozen());

        // Elapse more time than the freeze duration
        time_service.advance_secs(timeout_freeze_duration_secs + 1);

        // Verify that the prefetching value is no longer frozen
        assert!(!dynamic_prefetching_state.is_prefetching_value_frozen());
    }

    #[test]
    fn test_get_max_concurrent_requests_disabled() {
        // Create a data streaming service config with dynamic prefetching disabled
        let max_concurrent_requests = 10;
        let dynamic_prefetching_config = DynamicPrefetchingConfig {
            enable_dynamic_prefetching: false,
            ..Default::default()
        };
        let data_streaming_service_config = DataStreamingServiceConfig {
            max_concurrent_requests,
            dynamic_prefetching: dynamic_prefetching_config,
            ..Default::default()
        };

        // Create a new dynamic prefetching state
        let mut dynamic_prefetching_state =
            DynamicPrefetchingState::new(data_streaming_service_config, TimeService::mock());

        // Verify that dynamic prefetching is disabled
        assert!(!dynamic_prefetching_state.is_dynamic_prefetching_enabled());

        // Create a stream engine for transactions or outputs
        let stream_engine =
            create_transactions_or_outputs_stream_engine(data_streaming_service_config);

        // Verify that the max concurrent requests is the static config value
        verify_max_concurrent_requests(
            &mut dynamic_prefetching_state,
            &stream_engine,
            max_concurrent_requests,
        );

        // Increase the max concurrent requests several times and verify the value
        for _ in 0..10 {
            // Increase the max concurrent requests
            dynamic_prefetching_state.increase_max_concurrent_requests();

            // Verify that the max concurrent requests is still the static config value
            verify_max_concurrent_requests(
                &mut dynamic_prefetching_state,
                &stream_engine,
                max_concurrent_requests,
            );
        }

        // Decrease the max concurrent requests several times and verify the value
        for _ in 0..10 {
            // Decrease the max concurrent requests
            dynamic_prefetching_state.decrease_max_concurrent_requests();

            // Verify that the max concurrent requests is still the static config value
            verify_max_concurrent_requests(
                &mut dynamic_prefetching_state,
                &stream_engine,
                max_concurrent_requests,
            );
        }
    }

    #[test]
    fn test_get_max_concurrent_state_requests_disabled() {
        // Create a data streaming service config with dynamic prefetching disabled
        let max_concurrent_state_requests = 5;
        let dynamic_prefetching_config = DynamicPrefetchingConfig {
            enable_dynamic_prefetching: false,
            ..Default::default()
        };
        let data_streaming_service_config = DataStreamingServiceConfig {
            max_concurrent_state_requests,
            dynamic_prefetching: dynamic_prefetching_config,
            ..Default::default()
        };

        // Create a new dynamic prefetching state
        let mut dynamic_prefetching_state =
            DynamicPrefetchingState::new(data_streaming_service_config, TimeService::mock());

        // Verify that dynamic prefetching is disabled
        assert!(!dynamic_prefetching_state.is_dynamic_prefetching_enabled());

        // Create a stream engine for states
        let stream_engine = create_state_stream_engine(data_streaming_service_config);

        // Verify that the max concurrent state requests is the static config value
        verify_max_concurrent_requests(
            &mut dynamic_prefetching_state,
            &stream_engine,
            max_concurrent_state_requests,
        );

        // Increase the max concurrent requests several times and verify the value
        for _ in 0..10 {
            // Increase the max concurrent requests
            dynamic_prefetching_state.increase_max_concurrent_requests();

            // Verify that the max concurrent requests is still the static config value
            verify_max_concurrent_requests(
                &mut dynamic_prefetching_state,
                &stream_engine,
                max_concurrent_state_requests,
            );
        }

        // Decrease the max concurrent requests several times
        for _ in 0..10 {
            // Decrease the max concurrent requests
            dynamic_prefetching_state.decrease_max_concurrent_requests();

            // Verify that the max concurrent requests is still the static config value
            verify_max_concurrent_requests(
                &mut dynamic_prefetching_state,
                &stream_engine,
                max_concurrent_state_requests,
            );
        }
    }

    #[test]
    fn test_get_max_concurrent_requests() {
        // Create a data streaming service config with dynamic prefetching enabled
        let initial_prefetching_value = 5;
        let prefetching_value_increase = 1;
        let prefetching_value_decrease = 2;
        let dynamic_prefetching_config = DynamicPrefetchingConfig {
            enable_dynamic_prefetching: true,
            initial_prefetching_value,
            prefetching_value_increase,
            prefetching_value_decrease,
            ..Default::default()
        };
        let data_streaming_service_config = DataStreamingServiceConfig {
            dynamic_prefetching: dynamic_prefetching_config,
            ..Default::default()
        };

        // Create a new dynamic prefetching state
        let mut dynamic_prefetching_state =
            DynamicPrefetchingState::new(data_streaming_service_config, TimeService::mock());

        // Create a stream engine for transactions or outputs
        let stream_engine =
            create_transactions_or_outputs_stream_engine(data_streaming_service_config);

        // Verify that the max concurrent requests is the initial prefetching value
        verify_max_concurrent_requests(
            &mut dynamic_prefetching_state,
            &stream_engine,
            initial_prefetching_value,
        );

        // Increase the max concurrent requests several times and verify the value
        let mut expected_max_requests = initial_prefetching_value;
        for _ in 0..10 {
            // Increase the max concurrent requests
            dynamic_prefetching_state.increase_max_concurrent_requests();

            // Verify that the max concurrent requests has increased correctly
            expected_max_requests += prefetching_value_increase;
            verify_max_concurrent_requests(
                &mut dynamic_prefetching_state,
                &stream_engine,
                expected_max_requests,
            );
        }

        // Decrease the max concurrent requests several times and verify the value
        for _ in 0..3 {
            // Decrease the max concurrent requests
            dynamic_prefetching_state.decrease_max_concurrent_requests();

            // Verify that the max concurrent requests has decreased correctly
            expected_max_requests -= prefetching_value_decrease;
            verify_max_concurrent_requests(
                &mut dynamic_prefetching_state,
                &stream_engine,
                expected_max_requests,
            );
        }
    }

    #[test]
    fn test_get_max_concurrent_requests_max_value() {
        // Create a data streaming service config with dynamic prefetching enabled
        let initial_prefetching_value = 10;
        let prefetching_value_increase = 2;
        let max_prefetching_value = 30;
        let dynamic_prefetching_config = DynamicPrefetchingConfig {
            enable_dynamic_prefetching: true,
            initial_prefetching_value,
            prefetching_value_increase,
            max_prefetching_value,
            ..Default::default()
        };
        let data_streaming_service_config = DataStreamingServiceConfig {
            dynamic_prefetching: dynamic_prefetching_config,
            ..Default::default()
        };

        // Create a new dynamic prefetching state
        let mut dynamic_prefetching_state =
            DynamicPrefetchingState::new(data_streaming_service_config, TimeService::mock());

        // Create a stream engine for states
        let stream_engine = create_state_stream_engine(data_streaming_service_config);

        // Verify that the max concurrent requests is the initial prefetching value
        verify_max_concurrent_requests(
            &mut dynamic_prefetching_state,
            &stream_engine,
            initial_prefetching_value,
        );

        // Increase the max concurrent requests several times and verify the value
        let mut expected_max_requests = initial_prefetching_value;
        for _ in 0..10 {
            // Increase the max concurrent requests
            dynamic_prefetching_state.increase_max_concurrent_requests();

            // Verify that the max concurrent requests has increased correctly
            expected_max_requests += prefetching_value_increase;
            verify_max_concurrent_requests(
                &mut dynamic_prefetching_state,
                &stream_engine,
                expected_max_requests,
            );
        }

        // Increase the max concurrent requests many more times and verify the value
        for _ in 0..100 {
            // Increase the max concurrent requests
            dynamic_prefetching_state.increase_max_concurrent_requests();

            // Verify that the max concurrent requests has increased to the max value
            verify_max_concurrent_requests(
                &mut dynamic_prefetching_state,
                &stream_engine,
                max_prefetching_value,
            );
        }
    }

    #[test]
    fn test_get_max_concurrent_requests_min_value() {
        // Create a data streaming service config with dynamic prefetching enabled
        let initial_prefetching_value = 20;
        let prefetching_value_decrease = 1;
        let min_prefetching_value = 2;
        let dynamic_prefetching_config = DynamicPrefetchingConfig {
            enable_dynamic_prefetching: true,
            initial_prefetching_value,
            prefetching_value_decrease,
            min_prefetching_value,
            ..Default::default()
        };
        let data_streaming_service_config = DataStreamingServiceConfig {
            dynamic_prefetching: dynamic_prefetching_config,
            ..Default::default()
        };

        // Create a new dynamic prefetching state
        let mut dynamic_prefetching_state =
            DynamicPrefetchingState::new(data_streaming_service_config, TimeService::mock());

        // Create a stream engine for transactions or outputs
        let stream_engine =
            create_transactions_or_outputs_stream_engine(data_streaming_service_config);

        // Verify that the max concurrent requests is the initial prefetching value
        verify_max_concurrent_requests(
            &mut dynamic_prefetching_state,
            &stream_engine,
            initial_prefetching_value,
        );

        // Decrease the max concurrent requests several times and verify the value
        let mut expected_max_requests = initial_prefetching_value;
        for _ in 0..18 {
            // Decrease the max concurrent requests
            dynamic_prefetching_state.decrease_max_concurrent_requests();

            // Verify that the max concurrent requests has decreased correctly
            expected_max_requests -= prefetching_value_decrease;
            verify_max_concurrent_requests(
                &mut dynamic_prefetching_state,
                &stream_engine,
                expected_max_requests,
            );
        }

        // Decrease the max concurrent requests many more times and verify the value
        for _ in 0..100 {
            // Decrease the max concurrent requests
            dynamic_prefetching_state.decrease_max_concurrent_requests();

            // Verify that the max concurrent requests has decreased to the min value
            verify_max_concurrent_requests(
                &mut dynamic_prefetching_state,
                &stream_engine,
                min_prefetching_value,
            );
        }
    }

    #[test]
    fn test_prefetching_value_frozen() {
        // Create a data streaming service config with dynamic prefetching enabled
        let initial_prefetching_value = 5;
        let prefetching_value_increase = 1;
        let prefetching_value_decrease = 2;
        let timeout_freeze_duration_secs = 10;
        let dynamic_prefetching_config = DynamicPrefetchingConfig {
            enable_dynamic_prefetching: true,
            initial_prefetching_value,
            prefetching_value_increase,
            prefetching_value_decrease,
            timeout_freeze_duration_secs,
            ..Default::default()
        };
        let data_streaming_service_config = DataStreamingServiceConfig {
            dynamic_prefetching: dynamic_prefetching_config,
            ..Default::default()
        };

        // Create a new dynamic prefetching state
        let time_service = TimeService::mock();
        let mut dynamic_prefetching_state =
            DynamicPrefetchingState::new(data_streaming_service_config, time_service.clone());

        // Create a stream engine for transactions or outputs
        let stream_engine =
            create_transactions_or_outputs_stream_engine(data_streaming_service_config);

        // Increase the max concurrent requests several times and verify the value
        let mut expected_max_requests = initial_prefetching_value;
        for _ in 0..10 {
            // Increase the max concurrent requests
            dynamic_prefetching_state.increase_max_concurrent_requests();

            // Verify that the max concurrent requests has increased correctly
            expected_max_requests += prefetching_value_increase;
            verify_max_concurrent_requests(
                &mut dynamic_prefetching_state,
                &stream_engine,
                expected_max_requests,
            );
        }

        // Update the prefetcher state to simulate a timeout and verify the value is frozen
        dynamic_prefetching_state.decrease_max_concurrent_requests();
        assert!(dynamic_prefetching_state.is_prefetching_value_frozen());

        // Verify that the max concurrent requests has decreased correctly
        let mut expected_max_requests = expected_max_requests - prefetching_value_decrease;
        verify_max_concurrent_requests(
            &mut dynamic_prefetching_state,
            &stream_engine,
            expected_max_requests,
        );

        // Increase the max concurrent requests several more times
        for _ in 0..100 {
            // Increase the max concurrent requests
            dynamic_prefetching_state.increase_max_concurrent_requests();

            // Verify that the max concurrent requests has not changed (the value is frozen)
            verify_max_concurrent_requests(
                &mut dynamic_prefetching_state,
                &stream_engine,
                expected_max_requests,
            );
        }

        // Elapse less time than the freeze duration
        let time_service = time_service.into_mock();
        time_service.advance_secs(timeout_freeze_duration_secs - 1);

        // Increase the max concurrent requests and verify the prefetching value is still frozen
        dynamic_prefetching_state.increase_max_concurrent_requests();
        assert!(dynamic_prefetching_state.is_prefetching_value_frozen());
        verify_max_concurrent_requests(
            &mut dynamic_prefetching_state,
            &stream_engine,
            expected_max_requests,
        );

        // Decrease the max concurrent requests several times
        for _ in 0..3 {
            // Decrease the max concurrent requests
            dynamic_prefetching_state.decrease_max_concurrent_requests();

            // Verify that the max concurrent requests has decreased correctly
            expected_max_requests -= prefetching_value_decrease;
            verify_max_concurrent_requests(
                &mut dynamic_prefetching_state,
                &stream_engine,
                expected_max_requests,
            );
        }

        // Elapse more time than the freeze duration and verify the value is not frozen
        time_service.advance_secs(timeout_freeze_duration_secs + 1);
        assert!(!dynamic_prefetching_state.is_prefetching_value_frozen());

        // Increase the max concurrent requests several times and verify the value is not frozen
        for _ in 0..10 {
            // Increase the max concurrent requests
            dynamic_prefetching_state.increase_max_concurrent_requests();

            // Verify that the max concurrent requests has increased correctly
            expected_max_requests += prefetching_value_increase;
            verify_max_concurrent_requests(
                &mut dynamic_prefetching_state,
                &stream_engine,
                expected_max_requests,
            );
        }
    }

    /// Creates a stream engine for states
    fn create_state_stream_engine(
        data_streaming_service_config: DataStreamingServiceConfig,
    ) -> StreamEngine {
        // Create the stream request for states
        let stream_request = StreamRequest::GetAllStates(GetAllStatesRequest {
            version: 0,
            start_index: 0,
        });

        // Create and return the stream engine
        StreamEngine::new(
            data_streaming_service_config,
            &stream_request,
            &AdvertisedData::empty(),
        )
        .unwrap()
    }

    /// Creates a stream engine for transactions or outputs
    fn create_transactions_or_outputs_stream_engine(
        data_streaming_service_config: DataStreamingServiceConfig,
    ) -> StreamEngine {
        // Create the stream request for transactions or outputs
        let stream_request =
            StreamRequest::GetAllTransactionsOrOutputs(GetAllTransactionsOrOutputsRequest {
                start_version: 0,
                end_version: 100_000,
                proof_version: 100_000,
                include_events: true,
            });

        // Create and return the stream engine
        StreamEngine::new(
            data_streaming_service_config,
            &stream_request,
            &AdvertisedData::empty(),
        )
        .unwrap()
    }

    /// Verifies that the max concurrent requests is the expected value
    fn verify_max_concurrent_requests(
        dynamic_prefetching_state: &mut DynamicPrefetchingState,
        stream_engine: &StreamEngine,
        expected_max_requests: u64,
    ) {
        assert_eq!(
            dynamic_prefetching_state.get_max_concurrent_requests(stream_engine),
            expected_max_requests
        );
    }
}
