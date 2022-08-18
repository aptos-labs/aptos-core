// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_rest_client::types::deserialize_from_string;
use aptos_types::{account_address::AccountAddress, network_address::NetworkAddress};
use hex::FromHex;
use serde::{Deserialize, Serialize};
use serde_repr::*;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValidatorConfig {
    #[serde(deserialize_with = "from_hex_literal")]
    consensus_pubkey: Vec<u8>,
    #[serde(deserialize_with = "from_hex_literal")]
    network_addresses: Vec<u8>,
    #[serde(deserialize_with = "from_hex_literal")]
    fullnode_addresses: Vec<u8>,
    #[serde(deserialize_with = "deserialize_from_string")]
    validator_index: u64,
}

impl ValidatorConfig {
    pub fn validator_network_addresses(&self) -> Result<Vec<NetworkAddress>, bcs::Error> {
        bcs::from_bytes(&self.network_addresses)
    }
    pub fn fullnode_network_addresses(&self) -> Result<Vec<NetworkAddress>, bcs::Error> {
        bcs::from_bytes(&self.fullnode_addresses)
    }
}

/// Consensus information per validator, stored in ValidatorSet.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValidatorInfo {
    #[serde(deserialize_with = "deserialize_from_string")]
    addr: AccountAddress,
    #[serde(deserialize_with = "deserialize_from_string")]
    voting_power: u64,
    config: ValidatorConfig,
}

impl ValidatorInfo {
    pub fn account_address(&self) -> &AccountAddress {
        &self.addr
    }

    pub(crate) fn config(&self) -> &ValidatorConfig {
        &self.config
    }
}

#[derive(Clone, Copy, Debug, Deserialize_repr, Eq, PartialEq, Serialize_repr)]
#[repr(u8)]
pub enum ConsensusScheme {
    Ed25519 = 0,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValidatorSet {
    consensus_scheme: ConsensusScheme,
    active_validators: Vec<ValidatorInfo>,
    pending_inactive: Vec<ValidatorInfo>,
    pending_active: Vec<ValidatorInfo>,
}

impl ValidatorSet {
    pub fn payload(&self) -> impl Iterator<Item = &ValidatorInfo> {
        self.active_validators
            .iter()
            .chain(self.pending_inactive.iter())
    }
}

pub fn from_hex_literal<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    FromHex::from_hex(s.trim_start_matches("0x")).map_err(serde::de::Error::custom)
}
