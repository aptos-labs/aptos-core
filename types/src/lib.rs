// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

pub mod access_path;
pub mod account_address;
pub mod account_config;
pub mod block_info;
pub mod block_metadata;
pub mod block_metadata_ext;
pub mod chain_id;
pub mod contract_event;
pub mod dkg;
pub mod epoch_change;
pub mod epoch_state;
pub mod error;
pub mod event;
pub mod fee_statement;
pub mod function_info;
pub mod governance;
pub mod indexer;
pub mod jwks;
pub mod ledger_info;
pub mod mempool_status;
pub mod move_any;
pub mod move_fixed_point;
pub mod move_utils;
pub mod network_address;
pub mod nibble;
pub mod object_address;
pub mod on_chain_config;
pub mod proof;
#[cfg(any(test, feature = "fuzzing"))]
pub mod proptest_types;
pub mod randomness;
pub mod serde_helper;
pub mod stake_pool;
pub mod staking_contract;
pub mod state_proof;
#[cfg(any(test, feature = "fuzzing"))]
pub mod test_helpers;
pub mod timestamp;
pub mod transaction;
pub mod trusted_state;
pub mod utility_coin;
pub mod validator_config;
pub mod validator_info;
pub mod validator_performances;
pub mod validator_signer;
pub mod validator_txn;
pub mod validator_verifier;
pub mod vesting;
pub mod vm_status;
pub mod waypoint;
pub mod write_set;

pub use account_address::AccountAddress as PeerId;
pub use utility_coin::*;

pub mod aggregate_signature;
pub mod block_executor;
pub mod bytes;
pub mod delayed_fields;
pub mod keyless;
pub mod state_store;
#[cfg(test)]
mod unit_tests;
pub mod vm;
