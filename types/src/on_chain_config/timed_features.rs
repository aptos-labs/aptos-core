// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::chain_id::{ChainId, NamedChain};

// A placeholder that can be used to represent activation times that have not been determined.
const NOT_YET_SPECIFIED: u64 = END_OF_TIME; /* Thursday, December 31, 2099 11:59:59 PM */

pub const END_OF_TIME: u64 = 4102444799000; /* Thursday, December 31, 2099 11:59:59 PM */
#[derive(Debug, Clone, Copy)]
pub enum TimedFeatureFlag {
    VerifierLimitBackEdges,
    NativesAbortEarlyIfOutOfGas,
    VerifierMetering,
    MultiEd25519NativePublicKeyValidateGasFix,
    Ristretto255NativeFloatingPointFix,
    DisableInvariantViolationCheckInSwapLoc,
    EntryTypeSizeLimit,
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
                // During replay we want to have metering on but none of the other new features
                VerifierMetering => true,
                VerifierLimitBackEdges => false,
                // Disable the early-abort on out-of-gas in the installed safe natives, so we can test historical TXNs replay the same way.
                //NativesAbortEarlyIfOutOfGas => false,
                // Do not install the new safe native for Ristretto255 MSM, since it returns a different gas cost and would abort the replay test.
                //Ristretto255NativeFloatingPointFix => false,
                // Do not install the new safe native for MultiEd25519 PK validation, since it returns a different gas cost and would abort the replay test.
                //MultiEd25519NativePublicKeyValidateGasFix => false,
                // Add overrides for replay here.
                _ => return None,
            },
            Testing => !matches!(flag, VerifierLimitBackEdges), // Activate all flags but not legacy back edges
        })
    }
}

#[derive(Debug, Clone)]
pub struct TimedFeatures {
    inner: TimedFeaturesImpl,
    override_: Option<TimedFeatureOverride>,
}

impl TimedFeatureFlag {
    pub const fn activation_time_on(&self, chain_id: &NamedChain) -> u64 {
        use NamedChain::*;
        use TimedFeatureFlag::*;

        match (self, chain_id) {
            (VerifierLimitBackEdges, TESTNET) => 1675792800000, /* Tuesday, February 7, 2023 10:00:00 AM GMT-08:00 */
            (VerifierLimitBackEdges, MAINNET) => 1680231600000,

            (VerifierMetering, TESTNET) => 1680231600000, /* Thu Mar 30 2023 20:00:00 GMT-0700 */
            (VerifierMetering, MAINNET) => 1680318000000, /* Fri Mar 31 2023 20:00:00 GMT-0700 */

            (NativesAbortEarlyIfOutOfGas, TESTNET) => 1680231600000, /* Thu Mar 30 2023 20:00:00 GMT-0700 */
            (NativesAbortEarlyIfOutOfGas, MAINNET) => 1680318000000, /* Fri Mar 31 2023 20:00:00 GMT-0700 */

            (MultiEd25519NativePublicKeyValidateGasFix, TESTNET) => 1680231600000, /* Thu Mar 30 2023 20:00:00 GMT-0700 */
            (MultiEd25519NativePublicKeyValidateGasFix, MAINNET) => 1680318000000, /* Fri Mar 31 2023 20:00:00 GMT-0700 */

            (Ristretto255NativeFloatingPointFix, TESTNET) => 1680231600000, /* Thu Mar 30 2023 20:00:00 GMT-0700 */
            (Ristretto255NativeFloatingPointFix, MAINNET) => 1680318000000, /* Fri Mar 31 2023 20:00:00 GMT-0700 */

            (DisableInvariantViolationCheckInSwapLoc, TESTNET) => NOT_YET_SPECIFIED,
            (DisableInvariantViolationCheckInSwapLoc, MAINNET) => NOT_YET_SPECIFIED,

            (EntryTypeSizeLimit, TESTNET) => 1680231600000, /* Thu Mar 30 2023 20:00:00 GMT-0700 */
            (EntryTypeSizeLimit, MAINNET) => 1680318000000, /* Fri Mar 31 2023 20:00:00 GMT-0700 */
            // If unspecified, a timed feature is considered enabled from the very beginning of time.
            _ => 0,
        }
    }
}

impl TimedFeatures {
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
    pub fn is_enabled(&self, flag: TimedFeatureFlag) -> bool {
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
}
