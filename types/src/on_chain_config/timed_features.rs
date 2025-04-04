// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::chain_id::{ChainId, NamedChain};
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::America::Los_Angeles;
use serde::Serialize;
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{EnumCount as EnumCountMacro, EnumIter};

#[derive(Debug, EnumCountMacro, EnumIter, Clone, Copy, Eq, PartialEq)]
pub enum TimedFeatureFlag {
    DisableInvariantViolationCheckInSwapLoc,
    // Was always enabled.
    _LimitTypeTagSize,
    // Enabled on mainnet, cannot be disabled.
    _ModuleComplexityCheck,
    EntryCompatibility,
    ChargeBytesForPrints,

    // Fixes the bug of table natives not tracking the memory usage of the global values they create.
    FixMemoryUsageTracking,
}

/// Representation of features that are gated by the block timestamps.
#[derive(Debug, Clone)]
enum TimedFeaturesImpl {
    OnNamedChain {
        named_chain: NamedChain,
        // Unix Epoch timestamp in microseconds.
        timestamp_micros: u64,
    },
    EnableAll,
}

#[derive(Debug, Clone, Serialize)]
pub enum TimedFeatureOverride {
    Replay,
    Testing,
}

impl TimedFeatureOverride {
    #[allow(unused, clippy::match_single_binding)]
    const fn get_override(&self, flag: TimedFeatureFlag) -> Option<bool> {
        use TimedFeatureFlag::*;
        use TimedFeatureOverride::*;

        Some(match self {
            Replay => match flag {
                _LimitTypeTagSize => true,
                _ModuleComplexityCheck => true,
                // Add overrides for replay here.
                _ => return None,
            },
            Testing => match flag {
                EntryCompatibility => true,
                _ => return None, // Activate all flags
            },
        })
    }
}

const BEGINNING_OF_TIME: DateTime<Utc> = DateTime::UNIX_EPOCH;

impl TimedFeatureFlag {
    /// Returns the activation time of the feature on the given chain.
    pub fn activation_time_on(&self, chain_id: &NamedChain) -> DateTime<Utc> {
        use NamedChain::*;
        use TimedFeatureFlag::*;

        match (self, chain_id) {
            // Enabled from the beginning of time.
            (DisableInvariantViolationCheckInSwapLoc, TESTNET) => BEGINNING_OF_TIME,
            (DisableInvariantViolationCheckInSwapLoc, MAINNET) => BEGINNING_OF_TIME,

            // Note: These have been enabled since the start due to a bug.
            (_LimitTypeTagSize, TESTNET) => BEGINNING_OF_TIME,
            (_LimitTypeTagSize, MAINNET) => BEGINNING_OF_TIME,

            (_ModuleComplexityCheck, TESTNET) => Los_Angeles
                .with_ymd_and_hms(2024, 6, 25, 16, 0, 0)
                .unwrap()
                .with_timezone(&Utc),
            (_ModuleComplexityCheck, MAINNET) => Los_Angeles
                .with_ymd_and_hms(2024, 7, 3, 12, 0, 0)
                .unwrap()
                .with_timezone(&Utc),

            (EntryCompatibility, TESTNET) => Los_Angeles
                .with_ymd_and_hms(2024, 11, 6, 12, 0, 0)
                .unwrap()
                .with_timezone(&Utc),
            (EntryCompatibility, MAINNET) => Los_Angeles
                .with_ymd_and_hms(2024, 11, 12, 12, 0, 0)
                .unwrap()
                .with_timezone(&Utc),

            // Note: Activation time set to 1 hour after the beginning of time
            //       so we can test the old and new behaviors in tests.
            (FixMemoryUsageTracking, TESTING) => Utc.with_ymd_and_hms(1970, 1, 1, 1, 0, 0).unwrap(),
            (FixMemoryUsageTracking, TESTNET) => Los_Angeles
                .with_ymd_and_hms(2025, 3, 7, 12, 0, 0)
                .unwrap()
                .with_timezone(&Utc),
            (FixMemoryUsageTracking, MAINNET) => Los_Angeles
                .with_ymd_and_hms(2025, 3, 11, 17, 0, 0)
                .unwrap()
                .with_timezone(&Utc),

            (ChargeBytesForPrints, TESTNET) => Los_Angeles
                .with_ymd_and_hms(2025, 3, 7, 12, 0, 0)
                .unwrap()
                .with_timezone(&Utc),
            (ChargeBytesForPrints, MAINNET) => Los_Angeles
                .with_ymd_and_hms(2025, 3, 11, 17, 0, 0)
                .unwrap()
                .with_timezone(&Utc),

            // For chains other than testnet and mainnet, a timed feature is considered enabled from
            // the very beginning, if left unspecified.
            (_, TESTING | DEVNET | PREMAINNET) => BEGINNING_OF_TIME,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimedFeaturesBuilder {
    inner: TimedFeaturesImpl,
    override_: Option<TimedFeatureOverride>,
}

impl TimedFeaturesBuilder {
    /// `timestamp_micros` is a Unix Epoch timestamp in microseconds.
    pub fn new(chain_id: ChainId, timestamp_micros: u64) -> Self {
        let inner = match NamedChain::from_chain_id(&chain_id) {
            Ok(named_chain) => TimedFeaturesImpl::OnNamedChain {
                named_chain,
                timestamp_micros,
            },
            Err(_) => TimedFeaturesImpl::EnableAll, // Unknown chain => enable all features by default.
        };
        Self {
            inner,
            override_: None,
        }
    }

    pub fn enable_all() -> Self {
        Self {
            inner: TimedFeaturesImpl::EnableAll,
            override_: None,
        }
    }

    pub fn with_override_profile(self, profile: TimedFeatureOverride) -> Self {
        Self {
            inner: self.inner,
            override_: Some(profile),
        }
    }

    /// Determine whether the given feature should be enabled or not.
    fn is_enabled(&self, flag: TimedFeatureFlag) -> bool {
        use TimedFeaturesImpl::*;

        if let Some(override_) = &self.override_ {
            if let Some(enabled) = override_.get_override(flag) {
                return enabled;
            }
        }

        match &self.inner {
            OnNamedChain {
                named_chain,
                timestamp_micros,
            } => {
                *timestamp_micros >= flag.activation_time_on(named_chain).timestamp_micros() as u64
            },
            EnableAll => true,
        }
    }

    pub fn build(self) -> TimedFeatures {
        let mut enabled = [false; TimedFeatureFlag::COUNT];
        for flag in TimedFeatureFlag::iter() {
            enabled[flag as usize] = self.is_enabled(flag)
        }

        TimedFeatures(enabled)
    }
}

#[derive(Clone, Debug, Serialize, Eq, PartialEq)]
pub struct TimedFeatures([bool; TimedFeatureFlag::COUNT]);

impl TimedFeatures {
    pub fn is_enabled(&self, flag: TimedFeatureFlag) -> bool {
        self.0[flag as usize]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use claims::assert_ok;

    #[test]
    fn timed_features_override_is_serializable() {
        let replay = assert_ok!(bcs::to_bytes(&TimedFeatureOverride::Replay));
        let testing = assert_ok!(bcs::to_bytes(&TimedFeatureOverride::Testing));
        assert_ne!(replay, testing);
    }

    #[test]
    fn test_micros_conversion() {
        use NamedChain::*;

        assert_eq!(
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
                .unwrap()
                .timestamp_micros(),
            1_704_067_200_000_000
        );

        assert_eq!(
            Utc.with_ymd_and_hms(2024, 11, 15, 0, 0, 0)
                .unwrap()
                .timestamp_micros(),
            1_731_628_800_000_000
        );

        assert_eq!(
            TimedFeatureFlag::_ModuleComplexityCheck
                .activation_time_on(&TESTNET)
                .timestamp_micros(),
            1_719_356_400_000_000
        );
        assert_eq!(
            TimedFeatureFlag::_ModuleComplexityCheck
                .activation_time_on(&MAINNET)
                .timestamp_micros(),
            1_720_033_200_000_000
        );

        assert_eq!(
            TimedFeatureFlag::EntryCompatibility
                .activation_time_on(&TESTNET)
                .timestamp_micros(),
            1_730_923_200_000_000
        );
        assert_eq!(
            TimedFeatureFlag::EntryCompatibility
                .activation_time_on(&MAINNET)
                .timestamp_micros(),
            1_731_441_600_000_000
        );
    }

    #[test]
    fn test_timed_features_activation() {
        use TimedFeatureFlag::*;
        let jan_1_2024_micros = Utc
            .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
            .unwrap()
            .timestamp_micros() as u64;
        let nov_15_2024_micros = Utc
            .with_ymd_and_hms(2024, 11, 15, 0, 0, 0)
            .unwrap()
            .timestamp_micros() as u64;

        // Check testnet on Jan 1, 2024.
        let testnet_jan_1_2024 = TimedFeaturesBuilder::new(ChainId::testnet(), jan_1_2024_micros);
        assert!(
            testnet_jan_1_2024.is_enabled(DisableInvariantViolationCheckInSwapLoc),
            "DisableInvariantViolationCheckInSwapLoc should always be enabled"
        );
        assert!(
            testnet_jan_1_2024.is_enabled(_LimitTypeTagSize),
            "LimitTypeTagSize should always be enabled"
        );
        assert!(
            !testnet_jan_1_2024.is_enabled(_ModuleComplexityCheck),
            "ModuleComplexityCheck should be disabled on Jan 1, 2024 on testnet"
        );
        assert!(
            !testnet_jan_1_2024.is_enabled(EntryCompatibility),
            "EntryCompatibility should be disabled on Jan 1, 2024 on testnet"
        );
        // Check testnet on Nov 15, 2024.
        let testnet_nov_15_2024 = TimedFeaturesBuilder::new(ChainId::testnet(), nov_15_2024_micros);
        assert!(
            testnet_nov_15_2024.is_enabled(DisableInvariantViolationCheckInSwapLoc),
            "DisableInvariantViolationCheckInSwapLoc should always be enabled"
        );
        assert!(
            testnet_nov_15_2024.is_enabled(_LimitTypeTagSize),
            "LimitTypeTagSize should always be enabled"
        );
        assert!(
            testnet_nov_15_2024.is_enabled(_ModuleComplexityCheck),
            "ModuleComplexityCheck should be enabled on Nov 15, 2024 on testnet"
        );
        assert!(
            testnet_nov_15_2024.is_enabled(EntryCompatibility),
            "EntryCompatibility should be enabled on Nov 15, 2024 on testnet"
        );
        // Check mainnet on Jan 1, 2024.
        let mainnet_jan_1_2024 = TimedFeaturesBuilder::new(ChainId::mainnet(), jan_1_2024_micros);
        assert!(
            mainnet_jan_1_2024.is_enabled(DisableInvariantViolationCheckInSwapLoc),
            "DisableInvariantViolationCheckInSwapLoc should always be enabled"
        );
        assert!(
            mainnet_jan_1_2024.is_enabled(_LimitTypeTagSize),
            "LimitTypeTagSize should always be enabled"
        );
        assert!(
            !mainnet_jan_1_2024.is_enabled(_ModuleComplexityCheck),
            "ModuleComplexityCheck should be disabled on Jan 1, 2024 on mainnet"
        );
        assert!(
            !mainnet_jan_1_2024.is_enabled(EntryCompatibility),
            "EntryCompatibility should be disabled on Jan 1, 2024 on mainnet"
        );
        // Check mainnet on Nov 15, 2024.
        let mainnet_nov_15_2024 = TimedFeaturesBuilder::new(ChainId::mainnet(), nov_15_2024_micros);
        assert!(
            mainnet_nov_15_2024.is_enabled(DisableInvariantViolationCheckInSwapLoc),
            "DisableInvariantViolationCheckInSwapLoc should always be enabled"
        );
        assert!(
            mainnet_nov_15_2024.is_enabled(_LimitTypeTagSize),
            "LimitTypeTagSize should always be enabled"
        );
        assert!(
            mainnet_nov_15_2024.is_enabled(_ModuleComplexityCheck),
            "ModuleComplexityCheck should be enabled on Nov 15, 2024 on mainnet"
        );
        assert!(
            mainnet_nov_15_2024.is_enabled(EntryCompatibility),
            "EntryCompatibility should be enabled on Nov 15, 2024 on mainnet"
        );
    }
}
