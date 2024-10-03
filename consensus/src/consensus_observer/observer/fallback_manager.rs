// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::common::error::Error;
use aptos_config::config::ConsensusObserverConfig;
use aptos_storage_interface::DbReader;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::ledger_info::LedgerInfoWithSignatures;
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
            time_service,
        }
    }

    /// Verifies that the DB is continuing to sync and commit new data.
    /// If not, an error is returned, indicating that we should enter fallback mode.
    pub fn check_syncing_progress(&mut self) -> Result<(), Error> {
        // Get the current time and synced version from storage
        let time_now = self.time_service.now();
        let current_synced_version =
            self.db_reader
                .get_latest_ledger_info_version()
                .map_err(|error| {
                    Error::UnexpectedError(format!(
                        "Failed to read highest synced version: {:?}",
                        error
                    ))
                })?;

        // Verify that the synced version is increasing appropriately
        let (highest_synced_version, highest_version_timestamp) =
            self.highest_synced_version_and_time;
        if current_synced_version <= highest_synced_version {
            // The synced version hasn't increased. Check if we should enter fallback mode.
            let duration_since_highest_seen = time_now.duration_since(highest_version_timestamp);
            let fallback_threshold = Duration::from_secs(
                self.consensus_observer_config
                    .observer_fallback_sync_threshold_secs,
            );
            if duration_since_highest_seen > fallback_threshold {
                return Err(Error::ObserverProgressStopped(format!(
                    "Consensus observer is not making progress! Highest synced version: {}, elapsed: {:?}",
                    highest_synced_version, duration_since_highest_seen
                )));
            }
        }

        // Update the highest synced version and time
        self.highest_synced_version_and_time = (current_synced_version, time_now);

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
    use aptos_crypto::HashValue;
    use aptos_storage_interface::Result;
    use aptos_types::{
        aggregate_signature::AggregateSignature, block_info::BlockInfo, ledger_info::LedgerInfo,
        transaction::Version,
    };
    use claims::assert_matches;
    use mockall::mock;

    // This is a simple mock of the DbReader (it generates a MockDatabaseReader)
    mock! {
        pub DatabaseReader {}
        impl DbReader for DatabaseReader {
            fn get_latest_ledger_info_version(&self) -> Result<Version>;
        }
    }

    #[test]
    fn test_check_syncing_progress() {
        // Create a consensus observer config
        let observer_fallback_sync_threshold_secs = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            observer_fallback_sync_threshold_secs,
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
        mock_time_service.advance(Duration::from_secs(
            observer_fallback_sync_threshold_secs + 1,
        ));

        // Verify that the DB is still making sync progress (the next DB version is higher)
        let time_now = mock_time_service.now();
        assert!(fallback_manager.check_syncing_progress().is_ok());
        assert_eq!(
            fallback_manager.highest_synced_version_and_time,
            (second_synced_version, time_now)
        );

        // Elapse enough time to bypass the fallback threshold
        mock_time_service.advance(Duration::from_secs(
            observer_fallback_sync_threshold_secs + 1,
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
