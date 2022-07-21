// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network_address::NetworkAddress;
use aptos_crypto::bls12381;
use move_deps::move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

impl MoveStructType for ValidatorConfig {
    const MODULE_NAME: &'static IdentStr = ident_str!("stake");
    const STRUCT_NAME: &'static IdentStr = ident_str!("ValidatorConfig");
}

impl MoveResource for ValidatorConfig {}

#[derive(Debug, Deserialize, Serialize, Clone, Eq, PartialEq, Default)]
pub struct ValidatorOperatorConfigResource {
    pub human_name: Vec<u8>,
}

impl MoveStructType for ValidatorOperatorConfigResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("validator_operator_config");
    const STRUCT_NAME: &'static IdentStr = ident_str!("ValidatorOperatorConfig");
}

impl MoveResource for ValidatorOperatorConfigResource {}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ValidatorConfig {
    pub consensus_public_key: bls12381::PublicKey,
    /// This is an bcs serialized Vec<NetworkAddress>
    pub validator_network_addresses: Vec<u8>,
    /// This is an bcs serialized Vec<NetworkAddress>
    pub fullnode_network_addresses: Vec<u8>,
    pub validator_index: u64,
}

impl ValidatorConfig {
    pub fn new(
        consensus_public_key: bls12381::PublicKey,
        validator_network_addresses: Vec<u8>,
        fullnode_network_addresses: Vec<u8>,
        validator_index: u64,
    ) -> Self {
        ValidatorConfig {
            consensus_public_key,
            validator_network_addresses,
            fullnode_network_addresses,
            validator_index,
        }
    }

    pub fn fullnode_network_addresses(&self) -> Result<Vec<NetworkAddress>, bcs::Error> {
        bcs::from_bytes(&self.fullnode_network_addresses)
    }

    pub fn validator_network_addresses(&self) -> Result<Vec<NetworkAddress>, bcs::Error> {
        bcs::from_bytes(&self.validator_network_addresses)
    }
}
