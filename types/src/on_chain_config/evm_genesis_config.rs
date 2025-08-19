// Copyright (c) Supra Foundation
// SPDX-License-Identifier: Apache-2.0

use once_cell::sync::Lazy;

use super::OnChainConfig;
use crate::chain_id::ChainId;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use move_core_types::{
    ident_str, identifier::IdentStr, language_storage::TypeTag, move_resource::MoveStructType,
};

/// The Genesis configuration for EVM that can only be set once at genesis epoch.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct OnChainEvmGenesisConfig {
    /// The EVM chain ID, derived from the Move chain ID.
    pub chain_id: u64,
    /// The EOA configurations for pre-funding at genesis.
    pub eoas: Vec<GenesisEvmEOA>,
    /// The contract configurations for deployment at genesis.
    pub contracts: Vec<GenesisEvmContract>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GenesisEvmEOA {
    /// The address of the EOA to be funded
    pub address: String,
    /// The amount of native token to fund the EOA with.
    pub amount: u128,
}

/// The Creator address and nonce determines the contract' deployment address.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GenesisEvmContract {
    /// The creator address of the contract.
    pub creator: String,
    /// The nonce of the creator.
    pub nonce: u64,
    /// The amount of native token to fund the contract with.
    pub amount: u128,
    /// The bytecode of the contract to deploy.
    pub bytecode: Vec<u8>,
}

impl OnChainEvmGenesisConfig {
    /// Create a new OnChainEvmGenesisConfig with the given parameters.
    pub fn new(chain_id: ChainId, eoas: Vec<GenesisEvmEOA>, contracts: Vec<GenesisEvmContract>) -> Self {
        let chain_id = Self::derive_evm_chain_id_from_move_chain_id(chain_id);

        Self {
            chain_id,
            eoas,
            contracts,
        }
    }
    
    /// Derive the EVM chain ID from the Move chain ID.
    fn derive_evm_chain_id_from_move_chain_id(move_chain_id: ChainId) -> u64 {
        let chain_id = move_chain_id.id() as u64;
        chain_id << 32 | chain_id << 16 | chain_id
    }
}

/// This onchain config does not exist from genesis, until it is added by the governance proposal.
/// If the config is not found, Evm should not be enabled.
impl OnChainConfig for OnChainEvmGenesisConfig {
    const MODULE_IDENTIFIER: &'static str = "evm_genesis_config";
    const TYPE_IDENTIFIER: &'static str = "EvmGenesisConfig";

    /// The Move resource is
    /// ```ignore
    /// struct EvmGenesisConfig has copy, drop, store {
    ///    config: vector<u8>,
    /// }
    /// ```
    /// so we need two rounds of bcs deserilization to turn it back to EvmGenesisConfig
    fn deserialize_into_config(bytes: &[u8]) -> Result<Self> {
        let raw_bytes: Vec<u8> = bcs::from_bytes(bytes)?;
        bcs::from_bytes(&raw_bytes).map_err(|e| {
            anyhow!(
                "[on-chain evm genesis config] Failed to deserialize into config: {}",
                e
            )
        })
    }
}



/// Move event type `0x1::evm_genesis_config::EvmGenesisEvent` in rust.
/// See its doc in Move for more details.
#[derive(Serialize, Deserialize)]
pub struct EvmGenesisEvent {
}

impl MoveStructType for EvmGenesisEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("evm_genesis_config");
    const STRUCT_NAME: &'static IdentStr = ident_str!("EvmGenesisEvent");
}

pub static EVM_GENESIS_EVENT_MOVE_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| TypeTag::Struct(Box::new(EvmGenesisEvent::struct_tag())));
