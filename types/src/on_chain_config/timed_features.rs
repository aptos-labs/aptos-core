// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::chain_id::{ChainId, NamedChain};

// A placeholder that can be used to represent activation times that have not been determined.
const NOT_YET_SPECIFIED: u64 = END_OF_TIME; /* Thursday, December 31, 2099 11:59:59 PM */

pub const END_OF_TIME: u64 = 4102444799000; /* Thursday, December 31, 2099 11:59:59 PM */
#[derive(Debug, Clone, Copy)]
pub enum TimedFeatureFlag {
    VerifierLimitBackEdges,
    NativesAbortEarlyIfOutOfGas,
}

/// Representation of features that are gated by the block timestamps.
#[derive(Debug, Clone)]
pub enum TimedFeaturesImpl {
    OnNamedChain {
        named_chain: NamedChain,
        timestamp: u64,
    },
    EnableAll,
}

#[derive(Debug, Clone)]
pub struct TimedFeatures(TimedFeaturesImpl);

impl TimedFeatureFlag {
    pub const fn activation_time_on(&self, chain_id: &NamedChain) -> u64 {
        use NamedChain::*;
        use TimedFeatureFlag::*;

        match (self, chain_id) {
            (VerifierLimitBackEdges, TESTNET) => 1675792800000, /* Tuesday, February 7, 2023 10:00:00 AM GMT-08:00 */
            (VerifierLimitBackEdges, MAINNET) => NOT_YET_SPECIFIED,

            (NativesAbortEarlyIfOutOfGas, TESTNET) => 1676311200000, /* Monday, February 13, 2023 10:00:00 AM GMT-08:00 */
            (NativesAbortEarlyIfOutOfGas, MAINNET) => NOT_YET_SPECIFIED,

            // If unspecified, a timed feature is considered enabled from the very beginning of time.
            _ => 0,
        }
    }
}

impl TimedFeatures {
    pub fn new(chain_id: ChainId, timestamp: u64) -> Self {
        match NamedChain::from_chain_id(&chain_id) {
            Ok(named_chain) => Self(TimedFeaturesImpl::OnNamedChain {
                named_chain,
                timestamp,
            }),
            Err(_) => Self(TimedFeaturesImpl::EnableAll), // Unknown chain => enable all features by default.
        }
    }

    pub fn enable_all() -> Self {
        Self(TimedFeaturesImpl::EnableAll)
    }

    /// Determine whether the given feature should be enabled or not.
    pub fn is_enabled(&self, flag: TimedFeatureFlag) -> bool {
        use TimedFeaturesImpl::*;

        match &self.0 {
            OnNamedChain {
                named_chain,
                timestamp,
            } => {
                println!("{:?}", named_chain);
                println!("{}", timestamp);
                *timestamp >= flag.activation_time_on(named_chain)
            }
            EnableAll => true,
        }
    }
}
