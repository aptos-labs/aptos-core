// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::common::{
    error::Error,
    logging::{LogEntry, LogSchema},
};
use velor_config::config::ConsensusObserverConfig;
use velor_logger::warn;
use velor_storage_interface::DbReader;
use velor_time_service::{TimeService, TimeServiceTrait};
use velor_types::{ledger_info::LedgerInfoWithSignatures, transaction::Version};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// The manager for fallback mode in consensus observer
pub struct ObserverFallbackManager {
    // The configuration of the consensus observer
    consensus_observer_config: ConsensusObserverConfig,

    // A handle to storage (used to read the latest state and check progress)
    db_reader: Arc<dyn DbReader>,

    // The highest synced version we've seen from storage, along with the time at which it was seen
    highest_synced_version_and_time: (u64, Instant),

    // The time at which the fallback manager started running
    start_time: Instant,

    // The time service (used to check the storage update time)
    time_service: TimeService,
}

impl ObserverFallbackManager {
    pub fn new(
        consensus_observer_config: ConsensusObserverConfig,
        db_reader: Arc<dyn DbReader>,
        time_service: TimeService,
    ) -> Self {
        // Get the current time
        let time_now = time_service.now();

        // Create a new fallback manager
        Self {
            consensus_observer_config,
            db_reader,
            highest_synced_version_and_time: (0, time_now),
            start_time: time_now,
            time_service,
        }
    }

    /// Verifies that the DB is continuing to sync and commit new data, and that
    /// the node has not fallen too far behind the rest of the network.
    /// If not, an error is returned, indicating that we should enter fallback mode.
    pub fn check_syncing_progress(&mut self) -> Result<(), Error> {
        // If we're still within the startup period, we don't need to verify progress
        let time_now = self.time_service.now();
        let startup_period = Duration::from_millis(
            self.consensus_observer_config
                .observer_fallback_startup_period_ms,
        );
        if time_now.duration_since(self.start_time) < startup_period {
            return Ok(()); // We're still in the startup period
        }

        // Fetch the synced ledger info version from storage
        let latest_ledger_info_version =
            self.db_reader
                .get_latest_ledger_info_version()
                .map_err(|error| {
                    Error::UnexpectedError(format!(
                        "Failed to read highest synced version: {:?}",
                        error
                    ))
                })?;

        // Verify that the synced version is increasing appropriately
        self.verify_increasing_sync_versions(latest_ledger_info_version, time_now)?;

        // Verify that the sync lag is within acceptable limits
        self.verify_sync_lag_health(latest_ledger_info_version)
    }

    /// Verifies that the synced version is increasing appropriately. If not
    /// (i.e., too much time has passed without an increase), an error is returned.
    fn verify_increasing_sync_versions(
        &mut self,
        latest_ledger_info_version: Version,
        time_now: Instant,
    ) -> Result<(), Error> {
        // Verify that the synced version is increasing appropriately
        let (highest_synced_version, highest_version_timestamp) =
            self.highest_synced_version_and_time;
        if latest_ledger_info_version <= highest_synced_version {
            // The synced version hasn't increased. Check if we should enter fallback mode.
            let duration_since_highest_seen = time_now.duration_since(highest_version_timestamp);
            let fallback_threshold = Duration::from_millis(
                self.consensus_observer_config
                    .observer_fallback_progress_threshold_ms,
            );
            if duration_since_highest_seen > fallback_threshold {
                Err(Error::ObserverProgressStopped(format!(
                    "Consensus observer is not making progress! Highest synced version: {}, elapsed: {:?}",
                    highest_synced_version, duration_since_highest_seen
                )))
            } else {
                Ok(()) // We haven't passed the fallback threshold yet
            }
        } else {
            // The synced version has increased. Update the highest synced version and time.
            self.highest_synced_version_and_time = (latest_ledger_info_version, time_now);
            Ok(())
        }
    }

    /// Verifies that the sync lag is within acceptable limits. If not, an error is returned.
    fn verify_sync_lag_health(&self, latest_ledger_info_version: Version) -> Result<(), Error> {
        // Get the latest block timestamp from storage
        let latest_block_timestamp_usecs = match self
            .db_reader
            .get_block_timestamp(latest_ledger_info_version)
        {
            Ok(block_timestamp_usecs) => block_timestamp_usecs,
            Err(error) => {
                // Log a warning and return without entering fallback mode
                warn!(LogSchema::new(LogEntry::ConsensusObserver)
                    .message(&format!("Failed to read block timestamp: {:?}", error)));
                return Ok(());
            },
        };

        // Get the current time (in microseconds)
        let timestamp_now_usecs = self.time_service.now_unix_time().as_micros() as u64;

        // Calculate the block timestamp lag (saturating at 0)
        let timestamp_lag_usecs = timestamp_now_usecs.saturating_sub(latest_block_timestamp_usecs);
        let timestamp_lag_duration = Duration::from_micros(timestamp_lag_usecs);

        // Check if the sync lag is within acceptable limits
        let sync_lag_threshold_ms = self
            .consensus_observer_config
            .observer_fallback_sync_lag_threshold_ms;
        if timestamp_lag_duration > Duration::from_millis(sync_lag_threshold_ms) {
            return Err(Error::ObserverFallingBehind(format!(
                "Consensus observer is falling behind! Highest synced version: {}, sync lag: {:?}",
                latest_ledger_info_version, timestamp_lag_duration
            )));
        }

        Ok(())
    }

    /// Resets the syncing progress to the latest synced ledger info and current time
    pub fn reset_syncing_progress(&mut self, latest_synced_ledger_info: &LedgerInfoWithSignatures) {
        // Get the current time and highest synced version
        let time_now = self.time_service.now();
        let highest_synced_version = latest_synced_ledger_info.ledger_info().version();

        // Update the highest synced version and time
        self.highest_synced_version_and_time = (highest_synced_version, time_now);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use velor_crypto::HashValue;
    use velor_storage_interface::Result;
    use velor_types::{
        aggregate_signature::AggregateSignature, block_info::BlockInfo, ledger_info::LedgerInfo,
        transaction::Version,
    };
    use claims::assert_matches;
    use mockall::mock;

    // This is a simple mock of the DbReader (it generates a MockDatabaseReader)
    mock! {
        pub DatabaseReader {}
        impl DbReader for DatabaseReader {
            fn get_block_timestamp(&self, version: Version) -> Result<u64>;

            fn get_latest_ledger_info_version(&self) -> Result<Version>;
        }
    }

    #[test]
    fn test_verify_increasing_sync_versions() {
        // Create a consensus observer config
        let observer_fallback_progress_threshold_ms = 10_000;
        let consensus_observer_config = ConsensusObserverConfig {
            observer_fallback_startup_period_ms: 0, // Disable the startup period
            observer_fallback_progress_threshold_ms,
            ..ConsensusObserverConfig::default()
        };

        // Create a mock DB reader with expectations
        let first_synced_version = 1;
        let second_synced_version = 2;
        let mut mock_db_reader = MockDatabaseReader::new();
        mock_db_reader
            .expect_get_latest_ledger_info_version()
            .returning(move || Ok(first_synced_version))
            .times(1); // Only allow one call for the first version
        mock_db_reader
            .expect_get_latest_ledger_info_version()
            .returning(move || Ok(second_synced_version)); // Allow multiple calls for the second version
        mock_db_reader
            .expect_get_block_timestamp()
            .returning(move |_| Ok(u64::MAX)); // Return a dummy block timestamp

        // Create a new fallback manager
        let time_service = TimeService::mock();
        let mut fallback_manager = ObserverFallbackManager::new(
            consensus_observer_config,
            Arc::new(mock_db_reader),
            time_service.clone(),
        );

        // Verify that the DB is making sync progress and that the highest synced version is updated
        let mock_time_service = time_service.into_mock();
        assert!(fallback_manager.check_syncing_progress().is_ok());
        assert_eq!(
            fallback_manager.highest_synced_version_and_time,
            (first_synced_version, mock_time_service.now())
        );

        // Elapse enough time to bypass the fallback threshold
        mock_time_service.advance(Duration::from_millis(
            observer_fallback_progress_threshold_ms + 1,
        ));

        // Verify that the DB is still making sync progress (the next DB version is higher)
        let time_now = mock_time_service.now();
        assert!(fallback_manager.check_syncing_progress().is_ok());
        assert_eq!(
            fallback_manager.highest_synced_version_and_time,
            (second_synced_version, time_now)
        );

        // Elapse some amount of time (but not enough to bypass the fallback threshold)
        mock_time_service.advance(Duration::from_millis(
            observer_fallback_progress_threshold_ms - 1,
        ));

        // Verify that the DB is still making sync progress (the threshold hasn't been reached)
        assert!(fallback_manager.check_syncing_progress().is_ok());
        assert_eq!(
            fallback_manager.highest_synced_version_and_time,
            (second_synced_version, time_now)
        );

        // Elapse enough time to bypass the fallback threshold
        mock_time_service.advance(Duration::from_millis(
            observer_fallback_progress_threshold_ms + 1,
        ));

        // Verify that the DB is not making sync progress and that fallback mode should be entered
        assert_matches!(
            fallback_manager.check_syncing_progress(),
            Err(Error::ObserverProgressStopped(_))
        );
        assert_eq!(
            fallback_manager.highest_synced_version_and_time,
            (second_synced_version, time_now)
        );
    }

    #[test]
    fn test_verify_increasing_sync_versions_startup_period() {
        // Create a consensus observer config
        let observer_fallback_progress_threshold_ms = 10_000;
        let observer_fallback_startup_period_ms = 90_0000;
        let consensus_observer_config = ConsensusObserverConfig {
            observer_fallback_startup_period_ms,
            observer_fallback_progress_threshold_ms,
            ..ConsensusObserverConfig::default()
        };

        // Create a mock DB reader with expectations
        let initial_version = 0;
        let first_synced_version = 1;
        let second_synced_version = 2;
        let mut mock_db_reader = MockDatabaseReader::new();
        mock_db_reader
            .expect_get_latest_ledger_info_version()
            .returning(move || Ok(first_synced_version))
            .times(1); // Only allow one call for the first version
        mock_db_reader
            .expect_get_latest_ledger_info_version()
            .returning(move || Ok(second_synced_version)); // Allow multiple calls for the second version
        mock_db_reader
            .expect_get_block_timestamp()
            .returning(move |_| Ok(u64::MAX)); // Return a dummy block timestamp

        // Create a new fallback manager
        let time_service = TimeService::mock();
        let mut fallback_manager = ObserverFallbackManager::new(
            consensus_observer_config,
            Arc::new(mock_db_reader),
            time_service.clone(),
        );

        // Verify that syncing progress is not checked during the startup period
        let mock_time_service = time_service.into_mock();
        let time_now = mock_time_service.now();
        for _ in 0..5 {
            // Elapse enough time to bypass the fallback threshold
            mock_time_service.advance(Duration::from_millis(
                observer_fallback_progress_threshold_ms + 1,
            ));

            // Verify that the DB is still making sync progress (we're still in the startup period)
            assert!(fallback_manager.check_syncing_progress().is_ok());
            assert_eq!(
                fallback_manager.highest_synced_version_and_time,
                (initial_version, time_now)
            );
        }

        // Elapse enough time to bypass the startup period
        mock_time_service.advance(Duration::from_millis(
            observer_fallback_startup_period_ms + 1,
        ));

        // Verify that the DB is making sync progress and that the highest synced version is updated
        assert!(fallback_manager.check_syncing_progress().is_ok());
        assert_eq!(
            fallback_manager.highest_synced_version_and_time,
            (first_synced_version, mock_time_service.now())
        );

        // Elapse enough time to bypass the fallback threshold
        mock_time_service.advance(Duration::from_millis(
            observer_fallback_progress_threshold_ms + 1,
        ));

        // Verify that the DB is still making sync progress (the next DB version is higher)
        let time_now = mock_time_service.now();
        assert!(fallback_manager.check_syncing_progress().is_ok());
        assert_eq!(
            fallback_manager.highest_synced_version_and_time,
            (second_synced_version, time_now)
        );

        // Elapse enough time to bypass the fallback threshold
        mock_time_service.advance(Duration::from_millis(
            observer_fallback_progress_threshold_ms + 1,
        ));

        // Verify that the DB is not making sync progress and that fallback mode should be entered
        assert_matches!(
            fallback_manager.check_syncing_progress(),
            Err(Error::ObserverProgressStopped(_))
        );
        assert_eq!(
            fallback_manager.highest_synced_version_and_time,
            (second_synced_version, time_now)
        );
    }

    #[test]
    fn test_verify_sync_lag_health() {
        // Create a consensus observer config
        let observer_fallback_sync_lag_threshold_ms = 10_000;
        let consensus_observer_config = ConsensusObserverConfig {
            observer_fallback_startup_period_ms: 0, // Disable the startup period
            observer_fallback_progress_threshold_ms: 999_999_999, // Disable the progress check
            observer_fallback_sync_lag_threshold_ms,
            ..ConsensusObserverConfig::default()
        };

        // Create a mock DB reader with expectations
        let time_service = TimeService::mock();
        let latest_block_timestamp = time_service.now_unix_time().as_micros() as u64;
        let mut mock_db_reader = MockDatabaseReader::new();
        mock_db_reader
            .expect_get_latest_ledger_info_version()
            .returning(move || Ok(1));
        mock_db_reader
            .expect_get_block_timestamp()
            .returning(move |_| Ok(latest_block_timestamp));

        // Create a new fallback manager
        let mut fallback_manager = ObserverFallbackManager::new(
            consensus_observer_config,
            Arc::new(mock_db_reader),
            time_service.clone(),
        );

        // Verify that the DB is making sync progress and that the sync lag is acceptable
        assert!(fallback_manager.check_syncing_progress().is_ok());

        // Elapse some amount of time (but not enough to bypass the sync lag threshold)
        let mock_time_service = time_service.into_mock();
        mock_time_service.advance(Duration::from_millis(
            observer_fallback_sync_lag_threshold_ms - 1,
        ));

        // Verify that the DB is making sync progress and that the sync lag is acceptable
        assert!(fallback_manager.check_syncing_progress().is_ok());

        // Elapse enough time to bypass the sync lag threshold
        mock_time_service.advance(Duration::from_millis(
            observer_fallback_sync_lag_threshold_ms + 1,
        ));

        // Verify that the sync lag is too high and that fallback mode should be entered
        assert_matches!(
            fallback_manager.check_syncing_progress(),
            Err(Error::ObserverFallingBehind(_))
        );
    }

    #[test]
    fn test_verify_sync_lag_health_startup_period() {
        // Create a consensus observer config
        let observer_fallback_sync_lag_threshold_ms = 10_000;
        let observer_fallback_startup_period_ms = 90_0000;
        let consensus_observer_config = ConsensusObserverConfig {
            observer_fallback_startup_period_ms,
            observer_fallback_progress_threshold_ms: 999_999_999, // Disable the progress check
            observer_fallback_sync_lag_threshold_ms,
            ..ConsensusObserverConfig::default()
        };

        // Create a mock DB reader with expectations
        let time_service = TimeService::mock();
        let latest_block_timestamp = time_service.now_unix_time().as_micros() as u64;
        let mut mock_db_reader = MockDatabaseReader::new();
        mock_db_reader
            .expect_get_latest_ledger_info_version()
            .returning(move || Ok(1));
        mock_db_reader
            .expect_get_block_timestamp()
            .returning(move |_| Ok(latest_block_timestamp));

        // Create a new fallback manager
        let mut fallback_manager = ObserverFallbackManager::new(
            consensus_observer_config,
            Arc::new(mock_db_reader),
            time_service.clone(),
        );

        // Verify that the DB is making sync progress and that we're still in the startup period
        let mock_time_service = time_service.into_mock();
        for _ in 0..5 {
            // Elapse enough time to bypass the sync lag threshold
            mock_time_service.advance(Duration::from_millis(
                observer_fallback_sync_lag_threshold_ms + 1,
            ));

            // Verify that the DB is still making sync progress (we're still in the startup period)
            assert!(fallback_manager.check_syncing_progress().is_ok());
        }

        // Elapse enough time to bypass the startup period
        mock_time_service.advance(Duration::from_millis(
            observer_fallback_startup_period_ms + 1,
        ));

        // Verify that the sync lag is too high and that fallback mode should be entered
        assert_matches!(
            fallback_manager.check_syncing_progress(),
            Err(Error::ObserverFallingBehind(_))
        );
    }

    #[test]
    fn test_reset_syncing_progress() {
        // Create a new fallback manager
        let consensus_observer_config = ConsensusObserverConfig::default();
        let mock_db_reader = MockDatabaseReader::new();
        let time_service = TimeService::mock();
        let mut fallback_manager = ObserverFallbackManager::new(
            consensus_observer_config,
            Arc::new(mock_db_reader),
            time_service.clone(),
        );

        // Verify the initial state of the highest synced version and time
        let mock_time_service = time_service.into_mock();
        assert_eq!(
            fallback_manager.highest_synced_version_and_time,
            (0, mock_time_service.now())
        );

        // Elapse some amount of time
        mock_time_service.advance(Duration::from_secs(10));

        // Reset the syncing progress to a new synced ledger info
        let block_version = 100;
        let block_info = BlockInfo::new(
            0,
            0,
            HashValue::zero(),
            HashValue::zero(),
            block_version,
            0,
            None,
        );
        let latest_synced_ledger_info = LedgerInfoWithSignatures::new(
            LedgerInfo::new(block_info, HashValue::zero()),
            AggregateSignature::empty(),
        );
        fallback_manager.reset_syncing_progress(&latest_synced_ledger_info);

        // Verify that the highest synced version and time have been updated
        assert_eq!(
            fallback_manager.highest_synced_version_and_time,
            (block_version, mock_time_service.now())
        );
    }
}
