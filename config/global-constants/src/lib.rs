// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! The purpose of this crate is to offer a single source of truth for the definitions of shared
//! constants within the codebase. This is useful because many different components within
//! Aptos often require access to global constant definitions (e.g., Safety Rules,
//! Key Manager, and Secure Storage). To avoid duplicating these definitions across crates
//! (and better allow these constants to be updated in a single location), we define them here.
#![forbid(unsafe_code)]

/// Definitions of global cryptographic keys (e.g., as held in secure storage)
pub const APTOS_ROOT_KEY: &str = "aptos_root";
pub const CONSENSUS_KEY: &str = "consensus";
pub const FULLNODE_NETWORK_KEY: &str = "fullnode_network";
pub const OPERATOR_ACCOUNT: &str = "operator_account";
pub const OPERATOR_KEY: &str = "operator";
pub const OWNER_ACCOUNT: &str = "owner_account";
pub const OWNER_KEY: &str = "owner";
pub const VALIDATOR_NETWORK_KEY: &str = "validator_network";

/// Definitions of global data items (e.g., as held in secure storage)
pub const SAFETY_DATA: &str = "safety_data";
pub const WAYPOINT: &str = "waypoint";
pub const GENESIS_WAYPOINT: &str = "genesis-waypoint";
pub const MOVE_MODULES: &str = "move_modules";
pub const MIN_PRICE_PER_GAS_UNIT: &str = "min_price_per_gas_unit";

// TODO(Gas): double check if this right
/// Definitions of global gas constants

#[cfg(any(test, feature = "testing"))]
pub const GAS_UNIT_PRICE: u64 = 0;
#[cfg(not(any(test, feature = "testing")))]
pub const GAS_UNIT_PRICE: u64 = 100;

pub const INITIAL_BALANCE: u64 = 100_000_000;
pub const MAX_GAS_AMOUNT: u64 = 100_000;
pub const GAS_HEADROOM_NUMERATOR: u64 = 3;
pub const GAS_HEADROOM_DENOMINATOR: u64 = 2;

/// Gas costs are dynamic based on storage, so the simulation values need some headroom applied by
/// the user if using it to estimate gas
pub fn adjust_gas_headroom(gas_used: u64, max_possible_gas: u64) -> u64 {
    std::cmp::min(
        max_possible_gas,
        (gas_used.saturating_mul(GAS_HEADROOM_NUMERATOR)).saturating_div(GAS_HEADROOM_DENOMINATOR),
    )
}
