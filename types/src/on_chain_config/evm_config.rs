// Copyright (c) Supra Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use crate::chain_id::ChainId;

use super::OnChainConfig;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum OnChainEvmConfig {
    V1(EvmConfigV1),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct EvmConfigV1 {
    chain_id: u64,
}

impl OnChainEvmConfig {
    /// Create a new EvmConfigV1 with the given move chain_id.
    /// The EVM chain_id is derived from the move chain_id.
    ///   `evm_chain_id = move_chain_id << 32 | move_chain_id << 16 | move_chain_id`
    pub fn new_v1(chain_id: ChainId) -> Self {
        let chain_id = chain_id.id() as u64;
        let chain_id = chain_id << 32 | chain_id << 16 | chain_id;
        Self::V1(EvmConfigV1 { chain_id })
    }

    pub fn chain_id(&self) -> u64 {
        match self {
            Self::V1(config) => config.chain_id,
        }
    }
}


/// This onchain config does not exist from genesis, until it is added by the governance proposal.
/// If the config is not found, Evm should not be enabled.
impl OnChainConfig for OnChainEvmConfig {
    const MODULE_IDENTIFIER: &'static str = "evm_config";
    const TYPE_IDENTIFIER: &'static str = "EvmConfig";

    /// The Move resource is
    /// ```ignore
    /// struct EvmConfig has copy, drop, store {
    ///    config: vector<u8>,
    /// }
    /// ```
    /// so we need two rounds of bcs deserilization to turn it back to OnChainEvmConfig
    fn deserialize_into_config(bytes: &[u8]) -> Result<Self> {
        let raw_bytes: Vec<u8> = bcs::from_bytes(bytes)?;
        bcs::from_bytes(&raw_bytes)
            .map_err(|e| anyhow!("[on-chain config] Failed to deserialize into config: {}", e))
    }
}
