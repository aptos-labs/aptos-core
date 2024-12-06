// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::chain_id::{ChainId, NamedChain};
use serde::Serialize;
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{EnumCount as EnumCountMacro, EnumIter};

#[derive(Debug, EnumCountMacro, EnumIter, Clone, Copy, Eq, PartialEq)]
pub enum TimedFeatureFlag {
    DisableInvariantViolationCheckInSwapLoc,
    LimitTypeTagSize,
    ModuleComplexityCheck,
    EntryCompatibility,
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
                LimitTypeTagSize => true,
                ModuleComplexityCheck => true,
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

impl TimedFeatureFlag {
    /// Returns the activation time of the feature on the given chain.
    /// The time is specified as a Unix Epoch timestamp in microseconds.
    pub const fn activation_time_micros_on(&self, chain_id: &NamedChain) -> u64 {
        use NamedChain::*;
        use TimedFeatureFlag::*;

        match (self, chain_id) {
            // Enabled from the beginning of time.
            (DisableInvariantViolationCheckInSwapLoc, TESTNET) => 0,
            (DisableInvariantViolationCheckInSwapLoc, MAINNET) => 0,

            (ModuleComplexityCheck, TESTNET) => 1_719_356_400_000_000, /* Tuesday, June 21, 2024 16:00:00 AM GMT-07:00 */
            (ModuleComplexityCheck, MAINNET) => 1_720_033_200_000_000, /* Wednesday, July 3, 2024 12:00:00 AM GMT-07:00 */

            (EntryCompatibility, TESTNET) => 1_730_923_200_000_000, /* Wednesday, Nov 6, 2024 12:00:00 AM GMT-07:00 */
            (EntryCompatibility, MAINNET) => 1_731_441_600_000_000, /* Tuesday, Nov 12, 2024 12:00:00 AM GMT-07:00 */

            // If unspecified, a timed feature is considered enabled from the very beginning of time.
            _ => 0,
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
            } => *timestamp_micros >= flag.activation_time_micros_on(named_chain),
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
    fn test_timed_features_activation() {
        use TimedFeatureFlag::*;
        // Monday, Jan 01, 2024 12:00:00.000 AM GMT
        let jan_1_2024_micros: u64 = 1_704_067_200_000_000;
        // Friday, November 15, 2024 12:00:00 AM GMT
        let nov_15_2024_micros: u64 = 1_731_628_800_000_000;

        // Check testnet on Jan 1, 2024.
        let testnet_jan_1_2024 = TimedFeaturesBuilder::new(ChainId::testnet(), jan_1_2024_micros);
        assert!(
            testnet_jan_1_2024.is_enabled(DisableInvariantViolationCheckInSwapLoc),
            "DisableInvariantViolationCheckInSwapLoc should always be enabled"
        );
        assert!(
            testnet_jan_1_2024.is_enabled(LimitTypeTagSize),
            "LimitTypeTagSize should always be enabled"
        );
        assert!(
            !testnet_jan_1_2024.is_enabled(ModuleComplexityCheck),
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
            testnet_nov_15_2024.is_enabled(LimitTypeTagSize),
            "LimitTypeTagSize should always be enabled"
        );
        assert!(
            testnet_nov_15_2024.is_enabled(ModuleComplexityCheck),
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
            mainnet_jan_1_2024.is_enabled(LimitTypeTagSize),
            "LimitTypeTagSize should always be enabled"
        );
        assert!(
            !mainnet_jan_1_2024.is_enabled(ModuleComplexityCheck),
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
            mainnet_nov_15_2024.is_enabled(LimitTypeTagSize),
            "LimitTypeTagSize should always be enabled"
        );
        assert!(
            mainnet_nov_15_2024.is_enabled(ModuleComplexityCheck),
            "ModuleComplexityCheck should be enabled on Nov 15, 2024 on mainnet"
        );
        assert!(
            mainnet_nov_15_2024.is_enabled(EntryCompatibility),
            "EntryCompatibility should be enabled on Nov 15, 2024 on mainnet"
        );
    }
}
