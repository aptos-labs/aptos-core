// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    interface::AptosDataClientInterface,
    logging::{LogEntry, LogEvent, LogSchema},
    metrics,
};
use aptos_config::config::AptosDataClientConfig;
use aptos_logger::{info, sample, sample::SampleRate, warn};
use aptos_storage_interface::DbReader;
use aptos_time_service::{TimeService, TimeServiceTrait};
use futures::StreamExt;
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};

// Useful constants
const LATENCY_MONITOR_LOG_FREQ_SECS: u64 = 5;
const MAX_NUM_TRACKED_VERSION_ENTRIES: usize = 10_000;
const MAX_VERSION_LAG_TO_TOLERATE: u64 = 10_000;

/// A simple monitor that tracks the latencies taken to see
/// and sync new blockchain data (i.e., transactions).
pub struct LatencyMonitor {
    advertised_version_timestamps: BTreeMap<u64, (Instant, u64)>, // The timestamps when advertised versions were first seen
    caught_up_to_latest: bool, // Whether the node has ever caught up to the latest blockchain version
    data_client: Arc<dyn AptosDataClientInterface + Send + Sync>, // The data client through which to see advertised data
    monitor_loop_interval: Duration, // The interval between latency monitor loop executions
    storage: Arc<dyn DbReader>,      // The reader interface to storage
    time_service: TimeService,       // The service to monitor elapsed time
}

impl LatencyMonitor {
    pub fn new(
        data_client_config: AptosDataClientConfig,
        data_client: Arc<dyn AptosDataClientInterface + Send + Sync>,
        storage: Arc<dyn DbReader>,
        time_service: TimeService,
    ) -> Self {
        let monitor_loop_interval =
            Duration::from_millis(data_client_config.latency_monitor_loop_interval_ms);

        Self {
            advertised_version_timestamps: BTreeMap::new(),
            caught_up_to_latest: false,
            data_client,
            monitor_loop_interval,
            storage,
            time_service,
        }
    }

    /// Starts the latency monitor and periodically updates the latency metrics
    pub async fn start_latency_monitor(mut self) {
        info!(
            (LogSchema::new(LogEntry::LatencyMonitor)
                .message("Starting the Aptos data client latency monitor!"))
        );
        let loop_ticker = self.time_service.interval(self.monitor_loop_interval);
        futures::pin_mut!(loop_ticker);

        // Start the monitor
        loop {
            // Wait for the next round
            loop_ticker.next().await;

            // Get the highest synced version from storage
            let highest_synced_version = match self.storage.get_latest_version() {
                Ok(version) => version,
                Err(error) => {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(LATENCY_MONITOR_LOG_FREQ_SECS)),
                        warn!(
                            (LogSchema::new(LogEntry::LatencyMonitor)
                                .event(LogEvent::StorageReadFailed)
                                .message(&format!("Unable to read the highest synced version: {:?}", error)))
                        );
                    );
                    continue; // Continue to the next round
                },
            };

            // Update the latency metrics for all versions that we've now synced
            self.update_latency_metrics(highest_synced_version);

            // Get the highest advertised version from the global data summary
            let advertised_data = &self.data_client.get_global_data_summary().advertised_data;
            let highest_advertised_version = match advertised_data.highest_synced_ledger_info() {
                Some(ledger_info) => ledger_info.ledger_info().version(),
                None => {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(LATENCY_MONITOR_LOG_FREQ_SECS)),
                        warn!(
                            (LogSchema::new(LogEntry::LatencyMonitor)
                                .event(LogEvent::AggregateSummary)
                                .message("Unable to get the highest advertised version!"))
                        );
                    );
                    continue; // Continue to the next round
                },
            };

            // Update the advertised version timestamps
            self.update_advertised_version_timestamps(
                highest_synced_version,
                highest_advertised_version,
            );
        }
    }

    /// Updates the latency metrics for all versions that have now been synced
    fn update_latency_metrics(&mut self, highest_synced_version: u64) {
        // Split the advertised versions into synced and unsynced versions
        let unsynced_advertised_versions = self
            .advertised_version_timestamps
            .split_off(&(highest_synced_version + 1));

        // Update the metrics for all synced versions
        for (synced_version, (seen_time, seen_timestamp_usecs)) in
            self.advertised_version_timestamps.iter()
        {
            // Update the seen to synced latencies
            let duration_from_seen_to_synced = self.time_service.now().duration_since(*seen_time);
            metrics::observe_value_with_label(
                &metrics::SYNC_LATENCIES,
                metrics::SEEN_TO_SYNC_LATENCY_LABEL,
                duration_from_seen_to_synced.as_secs_f64(),
            );

            // Update the proposal latencies
            if let Ok(block_timestamp_usecs) = self.storage.get_block_timestamp(*synced_version) {
                // Update the propose to seen latencies
                if let Some(duration_from_propose_to_seen) =
                    calculate_duration_from_proposal(block_timestamp_usecs, *seen_timestamp_usecs)
                {
                    metrics::observe_value_with_label(
                        &metrics::SYNC_LATENCIES,
                        metrics::PROPOSE_TO_SEEN_LATENCY_LABEL,
                        duration_from_propose_to_seen.as_secs_f64(),
                    );
                }

                // Update the propose to synced latencies
                let timestamp_now_usecs = self.get_timestamp_now_usecs();
                if let Some(duration_from_propose_to_sync) =
                    calculate_duration_from_proposal(block_timestamp_usecs, timestamp_now_usecs)
                {
                    metrics::observe_value_with_label(
                        &metrics::SYNC_LATENCIES,
                        metrics::PROPOSE_TO_SYNC_LATENCY_LABEL,
                        duration_from_propose_to_sync.as_secs_f64(),
                    );
                }
            }
        }

        // Update the advertised versions with those we still need to sync
        self.advertised_version_timestamps = unsynced_advertised_versions;
    }

    /// Updates the advertised version timestamps by inserting any newly seen versions
    /// into the map and garbage collecting any old versions.
    fn update_advertised_version_timestamps(
        &mut self,
        highest_synced_version: u64,
        highest_advertised_version: u64,
    ) {
        // Check if we're still catching up to the latest version
        if !self.caught_up_to_latest {
            if highest_synced_version + MAX_VERSION_LAG_TO_TOLERATE >= highest_advertised_version {
                info!(
                    (LogSchema::new(LogEntry::LatencyMonitor)
                        .event(LogEvent::CaughtUpToLatest)
                        .message(
                            "We've caught up to the latest version! Starting the latency monitor."
                        ))
                );
                self.caught_up_to_latest = true; // We've caught up
            } else {
                return; // We're still catching up, so we shouldn't update the advertised version timestamps
            }
        }

        // If we're already synced with the highest advertised version, there's nothing to do
        if highest_synced_version >= highest_advertised_version {
            return;
        }

        // Get the current time and timestamp (note: we store both because
        // there isn't a clean way of converting between them when relying
        // on the time service).
        let time_now_instant = self.time_service.now();
        let timestamp_now_usecs = self.get_timestamp_now_usecs();

        // Insert the newly seen version into the advertised version timestamps
        self.advertised_version_timestamps.insert(
            highest_advertised_version,
            (time_now_instant, timestamp_now_usecs),
        );

        // If the map is too large, garbage collect the old versions
        while self.advertised_version_timestamps.len() > MAX_NUM_TRACKED_VERSION_ENTRIES {
            // Remove the lowest version from the map by popping the first
            // item. This is possible because BTreeMaps are sorted by key.
            self.advertised_version_timestamps.pop_first();
        }
    }

    /// Returns the current timestamp (in microseconds) since the Unix epoch
    fn get_timestamp_now_usecs(&self) -> u64 {
        self.time_service.now_unix_time().as_micros() as u64
    }
}

/// Calculates the duration between the propose timestamp and the given
/// timestamp. If the propose time is not in the past, this returns None.
///
/// Note: the propose timestamp and the given timestamp should both
/// be durations (in microseconds) since the Unix epoch.
fn calculate_duration_from_proposal(
    propose_timestamp_usecs: u64,
    given_timestamp_usecs: u64,
) -> Option<Duration> {
    if given_timestamp_usecs > propose_timestamp_usecs {
        Some(Duration::from_micros(
            given_timestamp_usecs - propose_timestamp_usecs,
        ))
    } else {
        // Log the error and return None
        sample!(
            SampleRate::Duration(Duration::from_secs(LATENCY_MONITOR_LOG_FREQ_SECS)),
            warn!(
                (LogSchema::new(LogEntry::LatencyMonitor)
                    .event(LogEvent::UnexpectedError)
                    .message("The propose timestamp is ahead of the given timestamp!"))
            );
        );
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        latency_monitor,
        latency_monitor::{
            calculate_duration_from_proposal, LatencyMonitor, MAX_NUM_TRACKED_VERSION_ENTRIES,
            MAX_VERSION_LAG_TO_TOLERATE,
        },
        tests::mock::{create_mock_data_client, create_mock_db_reader},
    };
    use aptos_config::config::AptosDataClientConfig;
    use aptos_time_service::{TimeService, TimeServiceTrait};
    use std::time::{Duration, Instant};

    #[test]
    fn test_calculate_duration_from_proposal() {
        // Test a valid duration (i.e., where proposal time is earlier than the given time)
        let propose_timestamp_usecs = 100;
        let given_timestamp_usecs = 200;
        let calculated_duration =
            calculate_duration_from_proposal(propose_timestamp_usecs, given_timestamp_usecs);
        assert_eq!(
            calculated_duration,
            Some(Duration::from_micros(
                given_timestamp_usecs - propose_timestamp_usecs
            ))
        );

        // Test an invalid duration (i.e., where proposal time is equal to the given time)
        let timestamp_usecs = 100_000;
        let calculated_duration =
            calculate_duration_from_proposal(timestamp_usecs, timestamp_usecs);
        assert_eq!(calculated_duration, None);

        // Test an invalid duration (i.e., where proposal time is after the given time)
        let propose_timestamp_usecs = 100_000_001;
        let given_timestamp_usecs = 100_000_000;
        let calculated_duration =
            calculate_duration_from_proposal(propose_timestamp_usecs, given_timestamp_usecs);
        assert_eq!(calculated_duration, None);
    }

    #[tokio::test]
    async fn test_advertised_version_timestamps() {
        // Create a latency monitor
        let (time_service, mut latency_monitor) = create_latency_monitor();

        // Verify the initial state
        assert!(!latency_monitor.caught_up_to_latest);
        verify_advertised_version_timestamps_length(&mut latency_monitor, 0);

        // Update the advertised version timestamps
        let highest_advertised_version = MAX_VERSION_LAG_TO_TOLERATE + 100;
        let highest_synced_version = 0;
        latency_monitor.update_advertised_version_timestamps(
            highest_synced_version,
            highest_advertised_version,
        );

        // Verify that we still haven't caught up (the sync lag is too large)
        let time_service = time_service.into_mock();
        assert!(!latency_monitor.caught_up_to_latest);
        verify_advertised_version_timestamps_length(&mut latency_monitor, 0);

        // Update the advertised version timestamps
        let mut highest_advertised_version = MAX_VERSION_LAG_TO_TOLERATE + 100;
        let highest_synced_version = 100;
        latency_monitor.update_advertised_version_timestamps(
            highest_synced_version,
            highest_advertised_version,
        );

        // Verify that we've finally caught up and started tracking latencies
        assert!(latency_monitor.caught_up_to_latest);
        verify_advertised_version_timestamps_length(&mut latency_monitor, 1);

        // Verify the timestamps of the highest advertised version
        let (time_now_instant, timestamp_now_usecs) =
            get_advertised_version_timestamps(&mut latency_monitor, &highest_advertised_version);
        assert_eq!(time_now_instant, time_service.now());
        assert_eq!(
            timestamp_now_usecs,
            time_service.now_unix_time().as_micros() as u64
        );

        // Elapse the time
        time_service.advance_ms(1000);

        // Update the advertised version timestamps again
        highest_advertised_version += 100;
        latency_monitor.update_advertised_version_timestamps(
            highest_synced_version,
            highest_advertised_version,
        );

        // Verify the number of tracked versions
        verify_advertised_version_timestamps_length(&mut latency_monitor, 2);

        // Verify the timestamps of the highest advertised version
        let (time_now_instant, timestamp_now_usecs) =
            get_advertised_version_timestamps(&mut latency_monitor, &highest_advertised_version);
        assert_eq!(time_now_instant, time_service.now());
        assert_eq!(
            timestamp_now_usecs,
            time_service.now_unix_time().as_micros() as u64
        );
    }

    #[tokio::test]
    async fn test_advertised_version_timestamps_garbage_collection() {
        // Create a latency monitor (and mark it as caught up)
        let (time_service, mut latency_monitor) = create_latency_monitor();
        latency_monitor.caught_up_to_latest = true;

        // Update the advertised versions many more times than the max
        let num_advertised_versions = MAX_NUM_TRACKED_VERSION_ENTRIES as u64 * 5;
        for advertised_version in 0..num_advertised_versions {
            latency_monitor.update_advertised_version_timestamps(0, advertised_version);
        }

        // Verify that we're tracking the max number of advertised version timestamps
        // (i.e., that garbage collection has kicked in).
        verify_advertised_version_timestamps_length(
            &mut latency_monitor,
            MAX_NUM_TRACKED_VERSION_ENTRIES as u64,
        );

        // Update the latency metrics and verify that the tracked version timestamps are empty
        latency_monitor.update_latency_metrics(num_advertised_versions);
        verify_advertised_version_timestamps_length(&mut latency_monitor, 0);

        // Update the advertised versions many more times than the max (again)
        let time_service = time_service.into_mock();
        let start_time_usecs = time_service.now_unix_time().as_micros() as u64;
        for advertised_version in 0..num_advertised_versions {
            // Elapse some time (1 ms)
            time_service.advance_ms(1);

            // Update the advertised version timestamps
            latency_monitor.update_advertised_version_timestamps(0, advertised_version);
        }

        // Verify the advertised version timestamps are correctly populated
        let lowest_tracked_version =
            num_advertised_versions - (MAX_NUM_TRACKED_VERSION_ENTRIES as u64);
        for advertised_version in lowest_tracked_version..num_advertised_versions {
            let (_, timestamp_now_usecs) =
                get_advertised_version_timestamps(&mut latency_monitor, &advertised_version);
            assert_eq!(
                timestamp_now_usecs,
                start_time_usecs + ((advertised_version + 1) * 1000)
            );
        }
    }

    #[tokio::test]
    async fn test_advertised_version_timestamps_split() {
        // Create a latency monitor (and mark it as caught up)
        let (time_service, mut latency_monitor) = create_latency_monitor();
        latency_monitor.caught_up_to_latest = true;

        // Update the advertised versions several times
        let time_service = time_service.into_mock();
        let num_advertised_versions = 100;
        for advertised_version in 0..num_advertised_versions {
            // Elapse some time (1 ms)
            time_service.advance_ms(1);

            // Update the advertised version timestamps
            latency_monitor.update_advertised_version_timestamps(0, advertised_version + 1);
        }

        // Verify that we're tracking the correct number of advertised version timestamps
        verify_advertised_version_timestamps_length(&mut latency_monitor, num_advertised_versions);

        // Update the latency metrics (we've only synced the first half of the advertised versions)
        let highest_synced_version = 50;
        latency_monitor.update_latency_metrics(highest_synced_version);

        // Verify that we're tracking the correct number of advertised version timestamps
        let expected_num_tracked_versions = 50;
        verify_advertised_version_timestamps_length(
            &mut latency_monitor,
            expected_num_tracked_versions,
        );

        // Update the latency metrics (we've now almost synced all advertised versions)
        let highest_synced_version = 98;
        latency_monitor.update_latency_metrics(highest_synced_version);

        // Verify that we're tracking the correct number of advertised version timestamps
        let expected_num_tracked_versions = 2;
        verify_advertised_version_timestamps_length(
            &mut latency_monitor,
            expected_num_tracked_versions,
        );

        // Update the latency metrics (we've now synced all advertised versions)
        let highest_synced_version = 100;
        latency_monitor.update_latency_metrics(highest_synced_version);

        // Verify that we're tracking the correct number of advertised version timestamps
        verify_advertised_version_timestamps_length(&mut latency_monitor, 0);

        // Update the advertised version timestamps (we're now synced to the advertised version)
        latency_monitor.update_advertised_version_timestamps(200, 200);

        // Verify that we're tracking the correct number of advertised version timestamps
        verify_advertised_version_timestamps_length(&mut latency_monitor, 0);
    }

    /// Creates a latency monitor for testing
    fn create_latency_monitor() -> (TimeService, LatencyMonitor) {
        let data_client_config = AptosDataClientConfig::default();
        let data_client = create_mock_data_client();
        let storage = create_mock_db_reader();
        let time_service = TimeService::mock();
        let latency_monitor = latency_monitor::LatencyMonitor::new(
            data_client_config,
            data_client.clone(),
            storage.clone(),
            time_service.clone(),
        );

        (time_service, latency_monitor)
    }

    /// Returns the advertised version timestamps for the given version
    fn get_advertised_version_timestamps(
        latency_monitor: &mut LatencyMonitor,
        highest_advertised_version: &u64,
    ) -> (Instant, u64) {
        let (time_now_instant, timestamp_now_usecs) = latency_monitor
            .advertised_version_timestamps
            .get(highest_advertised_version)
            .unwrap();

        (*time_now_instant, *timestamp_now_usecs)
    }

    /// Verifies that the length of the advertised version timestamps is correct
    fn verify_advertised_version_timestamps_length(
        latency_monitor: &mut LatencyMonitor,
        expected_length: u64,
    ) {
        assert_eq!(
            latency_monitor.advertised_version_timestamps.len(),
            expected_length as usize
        );
    }
}
