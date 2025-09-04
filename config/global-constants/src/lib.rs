// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The purpose of this crate is to offer a single source of truth for the definitions of shared
//! constants within the codebase. This is useful because many different components within
//! Velor often require access to global constant definitions (e.g., Safety Rules,
//! Key Manager, and Secure Storage). To avoid duplicating these definitions across crates
//! (and better allow these constants to be updated in a single location), we define them here.
#![forbid(unsafe_code)]

/// Definitions of global cryptographic keys (e.g., as held in secure storage)
pub const CONSENSUS_KEY: &str = "consensus";
pub const OWNER_ACCOUNT: &str = "owner_account";

/// Definitions of global data items (e.g., as held in secure storage)
pub const SAFETY_DATA: &str = "safety_data";
pub const WAYPOINT: &str = "waypoint";
pub const GENESIS_WAYPOINT: &str = "genesis-waypoint";

// TODO(Gas): double check if this right
// Definitions of global gas constants

#[cfg(any(test, feature = "testing"))]
pub const GAS_UNIT_PRICE: u64 = 0;
#[cfg(not(any(test, feature = "testing")))]
pub const GAS_UNIT_PRICE: u64 = 100;

#[cfg(any(test, feature = "testing"))]
pub const MAX_GAS_AMOUNT: u64 = 100_000_000;
#[cfg(not(any(test, feature = "testing")))]
pub const MAX_GAS_AMOUNT: u64 = 2_000_000;

pub const GAS_HEADROOM_NUMERATOR: u64 = 3;
pub const GAS_HEADROOM_DENOMINATOR: u64 = 2;

pub const DEFAULT_BUCKETS: &[u64] = &[0, 150, 300, 500, 1000, 3000, 5000, 10000, 100000, 1000000];

/// Gas costs are dynamic based on storage, so the simulation values need some headroom applied by
/// the user if using it to estimate gas
pub fn adjust_gas_headroom(gas_used: u64, max_possible_gas: u64) -> u64 {
    std::cmp::min(
        max_possible_gas,
        (gas_used.saturating_mul(GAS_HEADROOM_NUMERATOR)).saturating_div(GAS_HEADROOM_DENOMINATOR),
    )
}
