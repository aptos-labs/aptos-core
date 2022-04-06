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
pub const EXECUTION_KEY: &str = "execution";
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
