// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

pub mod access_path;
pub mod account_address;
pub mod account_config;
pub mod account_state;
pub mod block_info;
pub mod block_metadata;
pub mod block_metadata_ext;
pub mod chain_id;
pub mod contract_event;
pub mod dkg;
pub mod epoch_change;
pub mod epoch_state;
pub mod event;
pub mod executable;
pub mod fee_statement;
pub mod governance;
pub mod jwks;
pub mod ledger_info;
pub mod mempool_status;
pub mod move_any;
pub mod move_resource;
pub mod move_utils;
pub mod network_address;
pub mod nibble;
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

use crate::validator_verifier::ValidatorConsensusInfo;
pub use account_address::AccountAddress as PeerId;
use aptos_crypto::bls12381::bls12381_keys;
use move_core_types::account_address::AccountAddress;
pub use utility_coin::*;

pub mod account_view;
pub mod aggregate_signature;
pub mod aggregator;
pub mod block_executor;
pub mod bytes;
pub mod state_store;
#[cfg(test)]
mod unit_tests;
pub mod zkid;

/// Reflection of `0x1::types::ValidatorConsensusInfo` in rust.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValidatorConsensusInfoMoveStruct {
    pub addr: AccountAddress,
    pub pk_bytes: Vec<u8>,
    pub voting_power: u64,
}

impl From<ValidatorConsensusInfo> for ValidatorConsensusInfoMoveStruct {
    fn from(value: ValidatorConsensusInfo) -> Self {
        let ValidatorConsensusInfo {
            address,
            public_key,
            voting_power,
        } = value;
        Self {
            addr: address,
            pk_bytes: public_key.to_bytes().to_vec(),
            voting_power,
        }
    }
}

impl TryFrom<ValidatorConsensusInfoMoveStruct> for ValidatorConsensusInfo {
    type Error = anyhow::Error;

    fn try_from(value: ValidatorConsensusInfoMoveStruct) -> Result<Self, Self::Error> {
        let ValidatorConsensusInfoMoveStruct {
            addr,
            pk_bytes,
            voting_power,
        } = value;
        let public_key = bls12381_keys::PublicKey::try_from(pk_bytes.as_slice())?;
        Ok(Self::new(addr, public_key, voting_power))
    }
}
