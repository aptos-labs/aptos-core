// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::chain_id::{ChainId, NamedChain};
use serde::Serialize;
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{EnumCount as EnumCountMacro, EnumIter};

// A placeholder that can be used to represent activation times that have not been determined.
const NOT_YET_SPECIFIED: u64 = END_OF_TIME; /* Thursday, December 31, 2099 11:59:59 PM */

pub const END_OF_TIME: u64 = 4102444799000; /* Thursday, December 31, 2099 11:59:59 PM */
#[derive(Debug, EnumCountMacro, EnumIter, Clone, Copy, Eq, PartialEq)]
pub enum TimedFeatureFlag {
    DisableInvariantViolationCheckInSwapLoc,
    LimitTypeTagSize,
}

/// Representation of features that are gated by the block timestamps.
#[derive(Debug, Clone)]
enum TimedFeaturesImpl {
    OnNamedChain {
        named_chain: NamedChain,
        timestamp: u64,
    },
    EnableAll,
}

#[derive(Debug, Clone)]
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
                // Add overrides for replay here.
                _ => return None,
            },
            Testing => false, // Activate all flags
        })
    }
}

impl TimedFeatureFlag {
    pub const fn activation_time_on(&self, chain_id: &NamedChain) -> u64 {
        use NamedChain::*;
        use TimedFeatureFlag::*;

        match (self, chain_id) {
            (DisableInvariantViolationCheckInSwapLoc, TESTNET) => NOT_YET_SPECIFIED,
            (DisableInvariantViolationCheckInSwapLoc, MAINNET) => NOT_YET_SPECIFIED,

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
    pub fn new(chain_id: ChainId, timestamp: u64) -> Self {
        let inner = match NamedChain::from_chain_id(&chain_id) {
            Ok(named_chain) => TimedFeaturesImpl::OnNamedChain {
                named_chain,
                timestamp,
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
                timestamp,
            } => *timestamp >= flag.activation_time_on(named_chain),
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
